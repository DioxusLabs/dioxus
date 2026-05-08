pub fn MediaPlayerControls() -> Element {
    rsx! {
        div {
            // Control Panel Row
            nav { class: "h-12 border-b border-gray-200 dark:border-[#222] flex items-center justify-between px-6 shrink-0 bg-gray-50 dark:bg-[#000000]",
                // Right: Playback controls + Settings
                div { class: "flex items-center gap-2",
                    // Playback buttons - shown only when track is loaded
                    if let Some(track_id_str) = &current_track_id {
                        // Get current track from the tracks hashmap
                        {
                            let current_track = tracks().get(track_id_str).cloned();
                            let track_id_owned = track_id_str.clone();

                            rsx! {
                                // Play and Download buttons
                                if current_track.is_some() {
                                    {
                                        rsx! {
                                            // Play button - streams audio to device
                                            button {
                                                class: "px-2.5 py-1 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded transition-colors flex items-center gap-1.5 text-sm text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 cursor-pointer disabled:opacity-50",
                                                title: "Play track (streams audio)",
                                                disabled: is_buffering(),
                                                onclick: {
                                                    let tid = track_id_owned.clone();
                                                    // Capture output device for streaming
                                                    let output_device = current_track
                                                        .as_ref()
                                                        .and_then(|t| t.preferred_output.clone());
                                                    move |_| {
                                                        let tid = tid.clone();
                                                        let output_device = output_device.clone();
                                                        async move {
                                                            playback_result.set(None);
                                                            playback_error.set(None);
                                                            is_buffering.set(true);

                                                            let track_id: uuid::Uuid = tid.parse().unwrap_or_default();
                                                            let request = StreamAudioRequest {};

                                                            match start_audio_stream(track_id, request).await {
                                                                Ok(stream) => {
                                                                    // Use output device if available, otherwise fall back to default
                                                                    let device = output_device
                                                                        .map(|d| format!("{}/", d))
                                                                        .unwrap_or_else(|| "/dev/audio/default".to_string());

                                                                    // Connect to the audio stream endpoint
                                                                    let stream_url = format!(
                                                                        "audio://stream/{}@{}{}",
                                                                        stream.session_token,
                                                                        stream.server_host,
                                                                        device,
                                                                    );
                                                                    // Open the audio stream
                                                                    #[cfg(feature = "web")]
                                                                    if let Some(window) = web_sys::window() {
                                                                        let _ = window.location().set_href(&stream_url);
                                                                    }
                                                                    #[cfg(not(feature = "web"))]
                                                                    let _ = stream_url;
                                                                }
                                                                Err(e) => {
                                                                    playback_error
                                                                        .set(Some(format!("Failed to start stream: {}", e)));
                                                                    show_error_modal.set(true);
                                                                }
                                                            }
                                                            is_buffering.set(false);
                                                        }
                                                    }
                                                },
                                                Icon {
                                                    name: "play_arrow",
                                                    size: 16,
                                                    color: IconColor::Custom("#2563eb".to_string()),
                                                }
                                                if is_buffering() {
                                                    "Buffering..."
                                                } else if SHOW_CONTROL_TEXT {
                                                    "Play"
                                                }
                                            }

                                            // Download button - downloads track locally
                                            button {
                                                class: "px-2.5 py-1 hover:bg-gray-100 dark:hover:bg-[#252525] rounded transition-colors flex items-center gap-1.5 text-sm text-gray-600 dark:text-[#888] hover:text-gray-900 dark:hover:text-white cursor-pointer disabled:opacity-50",
                                                title: "Download track for offline playback",
                                                disabled: is_buffering(),
                                                onclick: {
                                                    let tid = track_id_owned.clone();
                                                    move |_| {
                                                        let tid = tid.clone();
                                                        async move {
                                                            playback_result.set(None);
                                                            playback_error.set(None);
                                                            is_buffering.set(true);
                                                            show_download_modal.set(true);

                                                            let track_id: uuid::Uuid = tid.parse().unwrap_or_default();
                                                            let request = DownloadTrackRequest {};

                                                            match download_track(track_id, request).await {
                                                                Ok(download) => {
                                                                    playback_result.set(Some(download));
                                                                }
                                                                Err(e) => {
                                                                    playback_error
                                                                        .set(Some(format!("Failed to download: {}", e)));
                                                                }
                                                            }
                                                            is_buffering.set(false);
                                                        }
                                                    }
                                                },
                                                Icon {
                                                    name: "download",
                                                    size: 16,
                                                    color: IconColor::Custom("#666".to_string()),
                                                }
                                                if is_buffering() {
                                                    "Downloading..."
                                                } else if SHOW_CONTROL_TEXT {
                                                    "Download"
                                                }
                                            }
                                        }
                                    }
                                }

                                // Fullscreen mode toggle
                                button {
                                    class: "px-2.5 py-1 hover:bg-gray-100 dark:hover:bg-[#252525] rounded transition-colors flex items-center gap-1.5 text-sm text-gray-600 dark:text-[#888] hover:text-gray-900 dark:hover:text-white mr-2 pr-3 border-r border-gray-200 dark:border-[#333] cursor-pointer",
                                    title: if (ctx.fullscreen)() { "Exit fullscreen" } else { "Enter fullscreen" },
                                    onclick: move |_| ctx.fullscreen.set(!(ctx.fullscreen)()),
                                    Icon {
                                        name: if (ctx.fullscreen)() { "fullscreen_exit" } else { "fullscreen" },
                                        size: 16,
                                        color: IconColor::Custom("#666".to_string()),
                                    }
                                    if SHOW_CONTROL_TEXT {
                                        if (ctx.fullscreen)() {
                                            "Exit Fullscreen"
                                        } else {
                                            "Fullscreen"
                                        }
                                    }
                                }

                                div { class: "flex items-center gap-1 mr-2 pr-3 border-r border-gray-200 dark:border-[#333]",
                                    button {
                                        class: "px-2 py-1 hover:bg-gray-100 dark:hover:bg-[#252525] rounded transition-colors flex items-center gap-1.5 text-sm text-gray-600 dark:text-[#888] hover:text-gray-900 dark:hover:text-white cursor-pointer",
                                        title: "Add to playlist",
                                        onclick: move |_| show_playlist_modal.set(true),
                                        Icon {
                                            name: "playlist_add",
                                            size: 16,
                                            color: IconColor::Custom("#666".to_string()),
                                        }
                                        if SHOW_CONTROL_TEXT {
                                            "Add to Playlist"
                                        }
                                    }

                                    if let Some(ref track) = current_track {
                                        {
                                            let track_id_for_action = track_id_owned.clone();
                                            match track.status {
                                                TrackStatus::Playing { .. } => rsx! {
                                                    button {
                                                        class: "px-2 py-1 hover:bg-gray-100 dark:hover:bg-[#252525] rounded transition-colors flex items-center gap-1.5 text-sm text-gray-600 dark:text-[#888] hover:text-gray-900 dark:hover:text-white cursor-pointer",
                                                        title: "Pause playback",
                                                        onclick: move |_| {
                                                            let tid = track_id_for_action.clone();
                                                            async move {
                                                                let track_id: uuid::Uuid = tid.parse().unwrap_or_default();
                                                                if let Err(e) = pause_track(track_id).await {
                                                                    error!("Failed to pause: {:?}", e);
                                                                }
                                                            }
                                                        },
                                                        Icon {
                                                            name: "pause",
                                                            size: 16,
                                                            color: IconColor::Custom("#666".to_string()),
                                                        }
                                                        if SHOW_CONTROL_TEXT {
                                                            "Pause"
                                                        }
                                                    }
                                                },
                                                TrackStatus::Paused { .. } => rsx! {
                                                    button {
                                                        class: "px-2 py-1 bg-green-600 hover:bg-green-500 text-white rounded transition-colors flex items-center gap-1.5 text-sm cursor-pointer",
                                                        title: "Resume playback",
                                                        onclick: move |_| {
                                                            let tid = track_id_for_action.clone();
                                                            async move {
                                                                let track_id: uuid::Uuid = tid.parse().unwrap_or_default();
                                                                if let Err(e) = resume_track(track_id).await {
                                                                    error!("Failed to resume: {:?}", e);
                                                                }
                                                            }
                                                        },
                                                        Icon { name: "play_arrow", size: 16, color: IconColor::Light }
                                                        if SHOW_CONTROL_TEXT {
                                                            "Resume"
                                                        }
                                                    }
                                                },
                                                _ => rsx! {},
                                            }
                                        }
                                    }

                                    {
                                        let track_id_for_stop = track_id_owned.clone();
                                        rsx! {
                                            button {
                                                class: "px-2 py-1 hover:bg-red-100 dark:hover:bg-red-900/30 rounded transition-colors flex items-center gap-1.5 text-sm text-red-600 dark:text-red-400 cursor-pointer",
                                                title: "Stop playback",
                                                onclick: move |_| {
                                                    let tid = track_id_for_stop.clone();
                                                    async move {
                                                        if let Err(e) = stop_track(tid).await {
                                                            error!("Failed to stop: {:?}", e);
                                                        }
                                                    }
                                                },
                                                Icon {
                                                    name: "stop",
                                                    size: 16,
                                                    color: IconColor::Custom("#dc2626".to_string()),
                                                }
                                                if SHOW_CONTROL_TEXT {
                                                    "Stop"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
