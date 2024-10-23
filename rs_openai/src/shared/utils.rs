use rand::{distributions::Alphanumeric, Rng};

pub fn is_stream(stream: Option<bool>) -> bool {
    if stream.is_some() && stream.unwrap() {
        return true;
    }

    false
}

pub fn generate_random_string(length: usize) -> String {
    let rng = rand::thread_rng();
    rng.sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
