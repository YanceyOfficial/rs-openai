pub fn is_stream(stream: Option<bool>) -> bool {
    if stream.is_some() && stream.unwrap() {
        return true;
    }

    false
}
