use crate::app::hello_world::HelloWorld;
use crate::freeswitch::adapter::FreeswitchTelephonyAdapter;
use crate::freeswitch::esl::EslClientConfig;

#[tokio::main]
async fn main() {
    println!("Dialer worker starting...");

    let telephony_port = FreeswitchTelephonyAdapter::new(EslClientConfig {
        host: "localhost".to_string(),
        port: 8021,
        password: "ClueCon".to_string(),
    });

    let hello_world = HelloWorld::new(Box::new(telephony_port));

    hello_world.start().await.unwrap();

    Ok(())
}
