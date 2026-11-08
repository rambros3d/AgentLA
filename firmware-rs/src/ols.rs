//! OpenBench Logic Sniffer (OLS / Sump) TCP server on port 5555.
//! Provides native 1-click PulseView and Sigrok desktop waveform captures up to 100 MHz.

use agentla_analyzer::ols::*;
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embedded_io_async::Write as _;

#[embassy_executor::task]
pub async fn ols_server_task(stack: Stack<'static>) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 2048];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(embassy_time::Duration::from_secs(60)));

        if let Err(e) = socket.accept(5555).await {
            defmt::error!("OLS TCP accept error: {:?}", e);
            embassy_time::Timer::after(embassy_time::Duration::from_millis(200)).await;
            continue;
        }

        defmt::info!("OLS/Sump client connected on 10.10.10.10:5555 (PulseView/Sigrok)");

        let mut parser = OlsParser::new();
        let mut rle_enabled = false;
        let mut sample_rate_hz = 1_000_000u32;
        let mut client_buf = [0u8; 128];
        let mut stream_buf = [0u8; 1024];

        'session: loop {
            // Read incoming commands
            match socket.read(&mut client_buf).await {
                Ok(0) => {
                    // Client disconnected
                    break 'session;
                }
                Ok(n) => {
                    for &b in &client_buf[..n] {
                        if let Some(cmd) = parser.process_byte(b) {
                            match cmd {
                                OlsCommand::Reset => {
                                    crate::analyzer::stop_continuous_stream();
                                    parser.reset();
                                }
                                OlsCommand::QueryId => {
                                    if socket.write_all(OLS_ID_RESPONSE).await.is_err() {
                                        break 'session;
                                    }
                                }
                                OlsCommand::QueryMetadata => {
                                    let mut meta = [0u8; 256];
                                    let meta_len = generate_ols_metadata(&mut meta);
                                    if socket.write_all(&meta[..meta_len]).await.is_err() {
                                        break 'session;
                                    }
                                }
                                OlsCommand::SetDivider { sample_rate_hz: rate, .. } => {
                                    sample_rate_hz = rate;
                                }
                                OlsCommand::SetFlags { flags } => {
                                    // Bit 2: RLE mode enabled
                                    rle_enabled = (flags & 0x04) != 0;
                                }
                                OlsCommand::SetReadCount { .. } => {}
                                OlsCommand::SetTriggerMask { .. } | OlsCommand::SetTriggerValue { .. } => {}
                                OlsCommand::Run => {
                                    let format = if rle_enabled {
                                        agentla_analyzer::FLAG_FORMAT_SUMP_RLE
                                    } else {
                                        agentla_analyzer::FLAG_FORMAT_BIT_DELTA
                                    };
                                    let _ = crate::analyzer::start_continuous_stream(8, format, sample_rate_hz);

                                    // Stream data to client
                                    let mut streamed_packets = 0;
                                    while streamed_packets < 100 {
                                        let bytes = crate::analyzer::read_stream_payload(&mut stream_buf);
                                        if bytes > 0 {
                                            if socket.write_all(&stream_buf[..bytes]).await.is_err() {
                                                break 'session;
                                            }
                                            streamed_packets += 1;
                                        } else {
                                            crate::analyzer::STREAM_SIGNAL.wait().await;
                                        }
                                    }
                                    crate::analyzer::stop_continuous_stream();
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    break 'session;
                }
            }
        }

        crate::analyzer::stop_continuous_stream();
        defmt::info!("OLS client disconnected from :5555");
    }
}
