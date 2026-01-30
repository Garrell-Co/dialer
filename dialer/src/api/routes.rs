use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::controller::ControllerCommand;
use crate::telephony::DestinationType;

use super::AppState;

#[derive(Deserialize)]
pub struct DialRequest {
    pub destination: DestinationType,
    pub from: String,
    pub caller_id_name: Option<String>,
}

#[derive(Deserialize)]
pub struct TransferRequest {
    pub destination: DestinationType,
}

pub fn call_routes() -> Router<AppState> {
    Router::new()
        .route("/calls", post(dial))
        .route("/calls/{id}", delete(hangup))
        .route("/calls/{id}/hold", post(hold))
        .route("/calls/{id}/resume", post(resume))
        .route("/calls/{id}/transfer", post(transfer))
}

async fn dial(
    State(state): State<AppState>,
    Json(req): Json<DialRequest>,
) -> StatusCode {
    let cmd = ControllerCommand::Dial {
        destination: req.destination,
        from: req.from,
        caller_id_name: req.caller_id_name,
    };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn hangup(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let cmd = ControllerCommand::Hangup { call_id: id };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn hold(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let cmd = ControllerCommand::Hold { call_id: id };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn resume(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let cmd = ControllerCommand::Resume { call_id: id };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn transfer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<TransferRequest>,
) -> StatusCode {
    let cmd = ControllerCommand::Transfer {
        call_id: id,
        destination: req.destination,
    };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
