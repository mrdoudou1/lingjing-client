use super::*;

#[test]
fn mock_chat_stream_honors_stop_signal() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime should build");
    let profile = GatewayProfile {
        id: "mock-default".into(),
        name: "Mock Gateway".into(),
        base_url: "mock://local".into(),
        protocol: "openai-compatible".into(),
        api_key_ref: "system-keychain:mock-default".into(),
        enabled: true,
        is_default: true,
        created_at: None,
        updated_at: None,
    };
    let (_stop_tx, stop_rx) = watch::channel(true);
    let mut deltas = 0;
    let result = runtime.block_on(GatewayHttpClient::default().chat_stream(
        &profile,
        None,
        "gpt-4.1",
        "hello",
        stop_rx,
        |_| {
            deltas += 1;
            Ok(())
        },
    ));
    assert_eq!(result, Err("CANCELED: chat stream stopped".into()));
    assert_eq!(deltas, 0);
}

#[test]
fn mock_audio_adapters_return_deterministic_results() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime should build");
    let profile = GatewayProfile {
        id: "mock-default".into(),
        name: "Mock Gateway".into(),
        base_url: "mock://local".into(),
        protocol: "openai-compatible".into(),
        api_key_ref: "system-keychain:mock-default".into(),
        enabled: true,
        is_default: true,
        created_at: None,
        updated_at: None,
    };
    let tts = AudioRequest {
        gateway_profile_id: "mock-default".into(),
        model_id: "mock-audio".into(),
        kind: "tts".into(),
        text: Some("hello".into()),
        source_file_name: None,
        source_file_base64: None,
        voice: Some("Aria".into()),
        language: None,
        format: "MP3".into(),
        speed: None,
    };
    let stt = AudioRequest {
        kind: "stt".into(),
        source_file_name: Some("clip.wav".into()),
        source_file_base64: Some("aGk=".into()),
        ..tts.clone()
    };
    runtime.block_on(async {
        let speech = GatewayHttpClient::default()
            .synthesize_speech(&profile, None, &tts)
            .await
            .unwrap();
        assert!(String::from_utf8(speech).unwrap().contains("hello"));
        let transcript = GatewayHttpClient::default()
            .transcribe_audio(&profile, None, &stt)
            .await
            .unwrap();
        assert!(transcript.text.contains("clip.wav"));
    });
}

#[test]
fn video_status_parser_accepts_common_gateway_fields() {
    let status = GatewayHttpClient::parse_video_status(serde_json::json!({
        "job_id": "remote-1",
        "status": "completed",
        "progress": 100,
        "output": { "url": "https://example.test/video.mp4" }
    }))
    .expect("status should parse");
    assert_eq!(status.remote_id, "remote-1");
    assert_eq!(status.status, "completed");
    assert_eq!(status.progress, 100.0);
    assert_eq!(
        status.result_url.as_deref(),
        Some("https://example.test/video.mp4")
    );
}
