pub fn DashboardLayout() -> Element {
    rsx! {
        div {
            // Tab Navigation Row
            nav { class: "h-12 border-b border-gray-200 dark:border-[#222] flex items-center justify-between px-6 shrink-0 bg-gray-50 dark:bg-[#000000]",
                // Right: VM controls + Launch button
                div { class: "flex items-center gap-2",
                    // VM Control buttons - shown only on running VM page
                    if let Some(vm_id_str) = &current_vm_id {
                        // Get current VM from the vms hashmap
                        {
                            let current_vm = vms().get(vm_id_str).cloned();
                            let vm_id_owned = vm_id_str.clone();

                            rsx! {
                                // Open in VS Code and Terminal buttons
                                if current_vm.is_some() {
                                    {
                                        rsx! {
                                            // VS Code button - creates tunnel then opens VS Code
                                            button {
                                                class: "px-2.5 py-1 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded transition-colors flex items-center gap-1.5 text-sm text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 cursor-pointer disabled:opacity-50",
                                                title: "Open in VS Code (creates SSH tunnel)",
                                                disabled: creating_ssh_tunnel(),
                                                onclick: {
                                                    let vid = vm_id_owned.clone();
                                                    // Capture checkout path for VS Code URL
                                                    let checkout_path = current_vm
                                                        .as_ref()
                                                        .and_then(|vm| vm.checkout_path.clone());
                                                    move |_| {
                                                        let vid = vid.clone();
                                                        let checkout_path = checkout_path.clone();
                                                        async move {
                                                            ssh_tunnel_result.set(None);
                                                            ssh_tunnel_error.set(None);
                                                            creating_ssh_tunnel.set(true);

                                                            let vm_id: uuid::Uuid = vid.parse().unwrap_or_default();
                                                            let request = CreateSshTunnelRequest {};

                                                            match create_ssh_tunnel(vm_id, request).await {
                                                                Ok(tunnel) => {
                                                                    // Use checkout path if available, otherwise fall back to home dir
                                                                    let path = checkout_path
                                                                        .map(|p| format!("{}/", p))
                                                                        .unwrap_or_else(|| "/home/admin/".to_string());

                                                                    // Open VS Code with the tunnel credentials
                                                                    let vscode_url = format!(
                                                                        "vscode://vscode-remote/ssh-remote+{}@{}{}",
                                                                        tunnel.tunnel_username,
                                                                        tunnel.ssh_host,
                                                                        path,
                                                                    );
                                                                    // Open the VS Code URL
                                                                    #[cfg(feature = "web")]
                                                                    if let Some(window) = web_sys::window() {
                                                                        let _ = window.location().set_href(&vscode_url);
                                                                    }
                                                                    #[cfg(not(feature = "web"))]
                                                                    let _ = vscode_url;
                                                                }
                                                                Err(e) => {
                                                                    ssh_tunnel_error
                                                                        .set(Some(format!("Failed to create tunnel: {}", e)));
                                                                    show_ssh_tunnel_modal.set(true);
                                                                }
                                                            }
                                                            creating_ssh_tunnel.set(false);
                                                        }
                                                    }
                                                },
                                                MaterialIcon {
                                                    name: "code",
                                                    size: 16,
                                                    color: MaterialIconColor::Custom("#2563eb".to_string()),
                                                }
                                                if creating_ssh_tunnel() {
                                                    "Creating..."
                                                } else if SHOW_VM_CONTROL_TEXT {
                                                    "Open in VS Code"
                                                }
                                            }

                                            // Terminal button - creates tunnel and shows SSH command
                                            button {
                                                class: "px-2.5 py-1 hover:bg-gray-100 dark:hover:bg-[#252525] rounded transition-colors flex items-center gap-1.5 text-sm text-gray-600 dark:text-[#888] hover:text-gray-900 dark:hover:text-white cursor-pointer disabled:opacity-50",
                                                title: "Create SSH tunnel for terminal access",
                                                disabled: creating_ssh_tunnel(),
                                                onclick: {
                                                    let vid = vm_id_owned.clone();
                                                    move |_| {
                                                        let vid = vid.clone();
                                                        async move {
                                                            ssh_tunnel_result.set(None);
                                                            ssh_tunnel_error.set(None);
                                                            creating_ssh_tunnel.set(true);
                                                            show_ssh_tunnel_modal.set(true);

                                                            let vm_id: uuid::Uuid = vid.parse().unwrap_or_default();
                                                            let request = CreateSshTunnelRequest {};

                                                            match create_ssh_tunnel(vm_id, request).await {
                                                                Ok(tunnel) => {
                                                                    ssh_tunnel_result.set(Some(tunnel));
                                                                }
                                                                Err(e) => {
                                                                    ssh_tunnel_error
                                                                        .set(Some(format!("Failed to create tunnel: {}", e)));
                                                                }
                                                            }
                                                            creating_ssh_tunnel.set(false);
                                                        }
                                                    }
                                                },
                                                MaterialIcon {
                                                    name: "terminal",
                                                    size: 16,
                                                    color: MaterialIconColor::Custom("#666".to_string()),
                                                }
                                                if creating_ssh_tunnel() {
                                                    "Creating..."
                                                } else if SHOW_VM_CONTROL_TEXT {
                                                    "Terminal"
                                                }
                                            }
                                        }
                                    }
                                }

                                // Theater mode toggle
                                button {
                                    class: "px-2.5 py-1 hover:bg-gray-100 dark:hover:bg-[#252525] rounded transition-colors flex items-center gap-1.5 text-sm text-gray-600 dark:text-[#888] hover:text-gray-900 dark:hover:text-white mr-2 pr-3 border-r border-gray-200 dark:border-[#333] cursor-pointer",
                                    title: if (ctx.theater_mode)() { "Exit theater mode" } else { "Enter theater mode" },
                                    onclick: move |_| ctx.theater_mode.set(!(ctx.theater_mode)()),
                                    MaterialIcon {
                                        name: if (ctx.theater_mode)() { "fullscreen_exit" } else { "fullscreen" },
                                        size: 16,
                                        color: MaterialIconColor::Custom("#666".to_string()),
                                    }
                                    if SHOW_VM_CONTROL_TEXT {
                                        if (ctx.theater_mode)() {
                                            "Exit Theater"
                                        } else {
                                            "Theater Mode"
                                        }
                                    }
                                }

                                div { class: "flex items-center gap-1 mr-2 pr-3 border-r border-gray-200 dark:border-[#333]",
                                    button {
                                        class: "px-2 py-1 hover:bg-gray-100 dark:hover:bg-[#252525] rounded transition-colors flex items-center gap-1.5 text-sm text-gray-600 dark:text-[#888] hover:text-gray-900 dark:hover:text-white cursor-pointer",
                                        title: "Take snapshot",
                                        onclick: move |_| show_snapshot_modal.set(true),
                                        MaterialIcon {
                                            name: "photo_camera",
                                            size: 16,
                                            color: MaterialIconColor::Custom("#666".to_string()),
                                        }
                                        if SHOW_VM_CONTROL_TEXT {
                                            "Snapshot"
                                        }
                                    }

                                    if let Some(ref vm) = current_vm {
                                        {
                                            let vm_id_for_action = vm_id_owned.clone();
                                            match vm.status {
                                                VmStatus::Running { .. } => rsx! {
                                                    button {
                                                        class: "px-2 py-1 hover:bg-gray-100 dark:hover:bg-[#252525] rounded transition-colors flex items-center gap-1.5 text-sm text-gray-600 dark:text-[#888] hover:text-gray-900 dark:hover:text-white cursor-pointer",
                                                        title: "Suspend VM",
                                                        onclick: move |_| {
                                                            let vid = vm_id_for_action.clone();
                                                            async move {
                                                                let vm_id: uuid::Uuid = vid.parse().unwrap_or_default();
                                                                if let Err(e) = suspend_vm(vm_id).await {
                                                                    error!("Failed to suspend VM: {:?}", e);
                                                                }
                                                            }
                                                        },
                                                        MaterialIcon {
                                                            name: "pause",
                                                            size: 16,
                                                            color: MaterialIconColor::Custom("#666".to_string()),
                                                        }
                                                        if SHOW_VM_CONTROL_TEXT {
                                                            "Suspend"
                                                        }
                                                    }
                                                },
                                                VmStatus::Suspended { .. } => rsx! {
                                                    button {
                                                        class: "px-2 py-1 bg-green-600 hover:bg-green-500 text-white rounded transition-colors flex items-center gap-1.5 text-sm cursor-pointer",
                                                        title: "Resume VM",
                                                        onclick: move |_| {
                                                            let vid = vm_id_for_action.clone();
                                                            async move {
                                                                let vm_id: uuid::Uuid = vid.parse().unwrap_or_default();
                                                                if let Err(e) = resume_vm(vm_id).await {
                                                                    error!("Failed to resume VM: {:?}", e);
                                                                }
                                                            }
                                                        },
                                                        MaterialIcon { name: "play_arrow", size: 16, color: MaterialIconColor::Light }
                                                        if SHOW_VM_CONTROL_TEXT {
                                                            "Resume"
                                                        }
                                                    }
                                                },
                                                _ => rsx! {},
                                            }
                                        }
                                    }

                                    {
                                        let vm_id_for_stop = vm_id_owned.clone();
                                        rsx! {
                                            button {
                                                class: "px-2 py-1 hover:bg-red-100 dark:hover:bg-red-900/30 rounded transition-colors flex items-center gap-1.5 text-sm text-red-600 dark:text-red-400 cursor-pointer",
                                                title: "Stop VM",
                                                onclick: move |_| {
                                                    let vid = vm_id_for_stop.clone();
                                                    async move {
                                                        if let Err(e) = shutdown_vm(vid).await {
                                                            error!("Failed to stop VM: {:?}", e);
                                                        }
                                                    }
                                                },
                                                MaterialIcon {
                                                    name: "stop",
                                                    size: 16,
                                                    color: MaterialIconColor::Custom("#dc2626".to_string()),
                                                }
                                                if SHOW_VM_CONTROL_TEXT {
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
