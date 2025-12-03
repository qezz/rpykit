pub fn format_thread_join_error_best_effort(err: &(dyn std::any::Any + Send)) -> String {
    let msg = match err.downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match err.downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "(failed to extract error)",
        },
    };

    msg.into()
}
