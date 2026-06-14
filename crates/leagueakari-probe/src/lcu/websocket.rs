use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use serde_json::Value;
use std::sync::Arc;
use tokio_tungstenite::{
    Connector, connect_async_tls_with_config,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};

use super::{
    analysis, auth, champ_select, champions::ChampionCatalog, client::LcuClient,
    connection::LcuConnection, live_client, output, print_champ_select_summary,
    print_draft_summary, print_json, teammate_performance,
};

#[derive(Debug)]
struct LcuEvent {
    uri: String,
    event_type: String,
    data: Value,
}

#[derive(Debug)]
struct AcceptAnyCertificate;

impl ServerCertVerifier for AcceptAnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

pub async fn watch(
    connection: &LcuConnection,
    client: &LcuClient,
    current_summoner: Option<&Value>,
    champion_catalog: &ChampionCatalog,
    raw: bool,
    json: bool,
) -> Result<()> {
    let url = format!("wss://127.0.0.1:{}/", connection.port);
    let mut request = url.into_client_request()?;
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&auth::basic_auth_header(&connection.password))?,
    );

    let crypto_provider = rustls::crypto::ring::default_provider();
    let tls_config = rustls::ClientConfig::builder_with_provider(crypto_provider.into())
        .with_safe_default_protocol_versions()
        .context("failed to configure rustls protocol versions")?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCertificate))
        .with_no_client_auth();
    let connector = Connector::Rustls(Arc::new(tls_config));
    let (mut socket, _) =
        connect_async_tls_with_config(request, None, false, Some(connector)).await?;

    socket
        .send(Message::Text(r#"[5,"OnJsonApiEvent"]"#.into()))
        .await?;

    if json {
        output::print_event(
            "watch_status",
            &output::status_message("listening", "LCU websocket connected"),
        )?;
    } else {
        println!();
        println!("watch mode: listening for LCU gameflow and champ-select events");
        println!("press Ctrl+C to stop");
    }

    let mut teammate_cache = teammate_performance::TeammatePerformanceCache::default();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                if json {
                    output::print_event(
                        "watch_status",
                        &output::status_message("stopped", "Ctrl+C received"),
                    )?;
                } else {
                    println!("watch mode stopped");
                }
                break;
            }
            message = socket.next() => {
                let Some(message) = message else {
                    if json {
                        output::print_event(
                            "watch_status",
                            &output::status_message("closed", "websocket closed"),
                        )?;
                    } else {
                        println!("watch mode ended: websocket closed");
                    }
                    break;
                };
                let message = message?;
                if let Message::Text(text) = message {
                    handle_text_message(
                        &text,
                        client,
                        current_summoner,
                        champion_catalog,
                        &mut teammate_cache,
                        raw,
                        json
                    ).await?;
                }
            }
        }
    }

    Ok(())
}

async fn handle_text_message(
    text: &str,
    client: &LcuClient,
    current_summoner: Option<&Value>,
    champion_catalog: &ChampionCatalog,
    teammate_cache: &mut teammate_performance::TeammatePerformanceCache,
    raw: bool,
    json: bool,
) -> Result<()> {
    let Some(event) = parse_lcu_event(text) else {
        return Ok(());
    };

    match event.uri.as_str() {
        "/lol-gameflow/v1/gameflow-phase" => {
            if json {
                output::print_event(
                    "watch_gameflow",
                    &serde_json::json!({
                        "lcu_event_type": event.event_type,
                        "phase": event.data
                    }),
                )?;
            } else {
                println!();
                println!(
                    "watch event: gameflow {} -> {}",
                    event.event_type, event.data
                );
            }

            if event.data.as_str() == Some("ChampSelect") {
                match client
                    .get_json::<Value>("/lol-champ-select/v1/session")
                    .await
                {
                    Ok(session) => {
                        emit_champ_select_snapshot(
                            client,
                            current_summoner,
                            champion_catalog,
                            teammate_cache,
                            &session,
                            "watch",
                            Some("phase_enter"),
                            raw,
                            json,
                        )
                        .await?;
                    }
                    Err(error) => {
                        if json {
                            output::print_event(
                                "champ_select_status",
                                &output::status_message(
                                    "retrying",
                                    &format!("ChampSelect session is not ready yet: {error}"),
                                ),
                            )?;
                        }
                    }
                }
            } else if matches!(event.data.as_str(), Some("GameStart" | "InProgress")) {
                if let Some(draft_state) =
                    live_client::fetch_in_progress_draft(champion_catalog).await
                {
                    emit_draft_snapshot(
                        client,
                        current_summoner,
                        champion_catalog,
                        teammate_cache,
                        &draft_state,
                        "live-client",
                        Some("phase_enter"),
                        json,
                    )
                    .await?;
                }
            }
        }
        "/lol-champ-select/v1/session" => {
            emit_champ_select_snapshot(
                client,
                current_summoner,
                champion_catalog,
                teammate_cache,
                &event.data,
                "watch",
                Some(event.event_type.as_str()),
                raw,
                json,
            )
            .await?;
        }
        _ => {}
    }

    Ok(())
}

async fn emit_champ_select_snapshot(
    client: &LcuClient,
    current_summoner: Option<&Value>,
    champion_catalog: &ChampionCatalog,
    teammate_cache: &mut teammate_performance::TeammatePerformanceCache,
    session: &Value,
    source: &str,
    event_type: Option<&str>,
    raw: bool,
    json: bool,
) -> Result<()> {
    let draft_state = champ_select::parse_draft_state("ChampSelect", session);
    let analysis = analysis::analyze_draft(&draft_state);

    if json {
        if raw {
            output::print_event("raw_champ_select_session", session)?;
        }
        emit_draft_snapshot(
            client,
            current_summoner,
            champion_catalog,
            teammate_cache,
            &draft_state,
            source,
            event_type,
            json,
        )
        .await?;
    } else {
        println!();
        println!(
            "watch event: champ select {}",
            event_type.unwrap_or("snapshot")
        );
        print_champ_select_summary(session);
        if raw {
            print_json("watch champ select session", session)?;
        }

        print_draft_summary(&draft_state, champion_catalog);
        print_json(
            "watch composition analysis",
            &serde_json::to_value(analysis)?,
        )?;
    }

    Ok(())
}

async fn emit_draft_snapshot(
    client: &LcuClient,
    current_summoner: Option<&Value>,
    champion_catalog: &ChampionCatalog,
    teammate_cache: &mut teammate_performance::TeammatePerformanceCache,
    draft_state: &super::models::DraftState,
    source: &str,
    event_type: Option<&str>,
    json: bool,
) -> Result<()> {
    let analysis = analysis::analyze_draft(draft_state);
    let teammate_performance = teammate_performance::analyze_teammates(
        client,
        draft_state,
        current_summoner,
        teammate_cache,
    )
    .await;

    if json {
        output::print_event(
            "draft_snapshot",
            &output::draft_snapshot(
                source,
                event_type,
                draft_state,
                &analysis,
                champion_catalog,
                &teammate_performance,
            ),
        )?;
    }

    Ok(())
}

fn parse_lcu_event(text: &str) -> Option<LcuEvent> {
    let value = serde_json::from_str::<Value>(text).ok()?;
    let values = value.as_array()?;
    if values.first()?.as_i64()? != 8 {
        return None;
    }

    let payload = values.get(2)?;
    Some(LcuEvent {
        uri: payload.get("uri")?.as_str()?.to_string(),
        event_type: payload
            .get("eventType")
            .and_then(Value::as_str)
            .unwrap_or("Unknown")
            .to_string(),
        data: payload.get("data").cloned().unwrap_or(Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lcu_json_api_event() {
        let event = parse_lcu_event(
            r#"[8,"OnJsonApiEvent",{"eventType":"Update","uri":"/lol-gameflow/v1/gameflow-phase","data":"ChampSelect"}]"#,
        )
        .unwrap();

        assert_eq!(event.uri, "/lol-gameflow/v1/gameflow-phase");
        assert_eq!(event.event_type, "Update");
        assert_eq!(event.data, Value::String("ChampSelect".to_string()));
    }

    #[test]
    fn ignores_non_event_payloads() {
        assert!(parse_lcu_event(r#"[5,"OnJsonApiEvent"]"#).is_none());
    }
}
