use std::time::Duration;
use tokio::sync::mpsc;
use anyhow::{Result, anyhow};

use crate::{telephony::TelephonyEvent};

use super::esl::{EslClientConfig, EslCommand, EslPort, EslEvent, EslConnector};


pub async fn connection_manager_task(
    connector: EslClientConfig,
    domain_tx: mpsc::Sender<TelephonyEvent>,
    mut cmd_rx: mpsc::Receiver<EslCommand>
) {
    loop {
        let connection_result = establish_connection(&connector).await;
        if let Err(e) = connection_result {
            tracing::error!("Connection failed: {}, retrying...", e);
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };

        let (esl, esl_rx) = connection_result.unwrap();

        let result = run_connection_until_failure(esl, esl_rx, &domain_tx, &mut cmd_rx).await;
        if let Err(_) = result {
            let _ = domain_tx.send(TelephonyEvent::TransportDown).await;
        }
    }
}


async fn establish_connection(
    connector: &EslClientConfig
) -> Result<(Box<dyn EslPort>, mpsc::Receiver<EslEvent>)> {
    let mut esl = connector.connect().await?;
    let esl_rx = esl.take_event_rx();
    Ok((esl, esl_rx))
}


async fn run_connection_until_failure(
    esl: Box<dyn EslPort>,
    mut esl_rx: mpsc::Receiver<EslEvent>,
    domain_tx: &mpsc::Sender<TelephonyEvent>,
    cmd_rx: &mut mpsc::Receiver<EslCommand>,
) -> Result<()> {
    domain_tx.send(TelephonyEvent::TransportUp).await?;

    loop {
        tokio::select! {
            event_result = esl_rx.recv() => {
                let result = handle_esl_event(event_result, domain_tx).await;
                match result {
                    Ok(()) => continue,
                    Err(_) => return Err(anyhow::anyhow!("Event channel closed")),
                }
            }
            cmd_result = cmd_rx.recv() => {
                match handle_command(cmd_result, &esl).await {
                    Ok(()) => continue,
                    Err(_) => return Err(anyhow::anyhow!("Command channel closed")),
                }
            }
        }
    }
}

// Handle a single ESL event
async fn handle_esl_event(
    event_result: Option<EslEvent>,
    domain_tx: &mpsc::Sender<TelephonyEvent>,
) -> Result<()> {
    let ev = event_result.ok_or_else(|| anyhow::anyhow!("ESL connection closed"))?;
    let telephony_event = convert_esl_event(ev)?;
    domain_tx.send(telephony_event).await
        .map_err(|_| anyhow::anyhow!("Domain event channel closed"))
}


// Convert ESL event to telephony event
fn convert_esl_event(ev: EslEvent) -> Result<TelephonyEvent> {
    let call_id = ev.headers.get("Unique-ID")
        .cloned()
        .unwrap_or_default();
    
    match ev.event_name.as_str() {
        "CHANNEL_CREATE" => Ok(TelephonyEvent::CallOffered { call_id }),
        "CHANNEL_HANGUP" => Ok(TelephonyEvent::CallEnded { call_id }),
        _ => Err(anyhow::anyhow!("Unhandled event type: {}", ev.event_name)),
    }
}


async fn handle_command(
    cmd_result: Option<EslCommand>,
    esl: &Box<dyn EslPort>,
) -> Result<()> {
    let cmd = cmd_result.ok_or_else(|| anyhow::anyhow!("Command channel closed"))?;
    match cmd {
        EslCommand::Api {cmd, reply} => {
            let result = esl.api(cmd).await?;
            tracing::info!("Command response: {:?}", result);

            reply.send(Ok(result)).map_err(|_| anyhow!("Reply receiver dropped"))?;
            Ok(())
        },
        EslCommand::SendRaw { lines } => {
            esl.send_raw(lines).await?;
            Ok(())
        },
        EslCommand::Close => {
            Ok(())
        }
    }
}

