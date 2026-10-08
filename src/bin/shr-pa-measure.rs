//! Offline bounded JSON entrypoint. No audio, capture or configuration application.
use std::io::Read;
fn main() {
    let result = (|| -> Result<serde_json::Value, String> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() > 1 {
            return Err("usage: shr-pa-measure [request.json|-]".into());
        }
        let reader: Box<dyn Read> = match args.first().map(String::as_str) {
            None | Some("-") => Box::new(std::io::stdin()),
            Some(path) => Box::new(std::fs::File::open(path).map_err(|e| e.to_string())?),
        };
        let mut bytes = Vec::new();
        reader
            .take(shr_pa::measurement::MAX_JSON_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        shr_pa::measurement::execute(shr_pa::measurement::decode_request(&bytes)?)
    })();
    match result {
        Ok(value) => println!("{value}"),
        Err(reason) => {
            println!(
                "{}",
                serde_json::json!({"contract":"C-PA-MEASUREMENT-REFUSAL","version":1,"status":"refused","reason":reason})
            );
            std::process::exit(2);
        }
    }
}
