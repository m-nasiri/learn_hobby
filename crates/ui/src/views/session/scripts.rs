pub(super) fn session_timer_script(
    timer_key: &str,
    timer_active: bool,
    show_timer: bool,
    soft_reminder: bool,
    auto_advance: bool,
    soft_secs: u32,
    auto_secs: u32,
) -> String {
    format!(
        r#"(function() {{
                    const root = document.getElementById("session-root");
                    const state = window.__learnSessionTimer || (window.__learnSessionTimer = {{
                        key: null,
                        seconds: 0,
                        autoFired: false,
                        id: null,
                    }});
                    if (!root) {{
                        if (state.id) {{
                            clearInterval(state.id);
                            state.id = null;
                        }}
                        state.key = null;
                        state.seconds = 0;
                        state.autoFired = false;
                        return;
                    }}
                    const key = {timer_key:?};
                    const active = {timer_active};
                    const showTimer = {show_timer};
                    const softReminder = {soft_reminder};
                    const autoAdvance = {auto_advance};
                    const softSecs = {soft_secs};
                    const autoSecs = {auto_secs};
                    const label = document.getElementById("session-timer-label");
                    const reminder = document.getElementById("session-soft-reminder");
                    if (state.key !== key) {{
                        state.key = key;
                        state.seconds = 0;
                        state.autoFired = false;
                    }}
                    const updateUi = () => {{
                        if (label) {{
                            if (showTimer && active) {{
                                const minutes = Math.floor(state.seconds / 60);
                                const seconds = String(state.seconds % 60).padStart(2, "0");
                                label.textContent = "Time: " + minutes + ":" + seconds;
                                label.hidden = false;
                            }} else {{
                                label.hidden = true;
                            }}
                        }}
                        if (reminder) {{
                            const show = softReminder && active && state.seconds >= softSecs;
                            reminder.hidden = !show;
                        }}
                    }};
                    updateUi();
                    if (!active) {{
                        if (state.id) {{
                            clearInterval(state.id);
                            state.id = null;
                        }}
                        return;
                    }}
                    if (!state.id) {{
                        state.id = setInterval(() => {{
                            if (!document.getElementById("session-root")) {{
                                clearInterval(state.id);
                                state.id = null;
                                return;
                            }}
                            state.seconds += 1;
                            updateUi();
                            if (autoAdvance && !state.autoFired && state.seconds >= autoSecs) {{
                                state.autoFired = true;
                                const btn = document.getElementById("session-reveal");
                                if (btn) btn.click();
                            }}
                        }}, 1000);
                    }}
                }})();"#,
        timer_key = timer_key,
        timer_active = timer_active,
        show_timer = show_timer,
        soft_reminder = soft_reminder,
        auto_advance = auto_advance,
        soft_secs = soft_secs,
        auto_secs = auto_secs,
    )
}
