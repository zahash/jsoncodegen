mod generated;

fn main() {
    let data: generated::ROOT =
        serde_json::from_reader(std::io::stdin()).expect("Failed to deserialize input");
    serde_json::to_writer_pretty(std::io::stdout(), &data).expect("Failed to serialize output");
}
