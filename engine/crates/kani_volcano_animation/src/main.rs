use std::{error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let input_path = Path::new("../../assets/models/fox.glb");
    let output_path = Path::new("../../assets/models/fox.json");

    let bytes = fs::read(input_path)?;

    let (document, buffers, images) =
        gltf::import("../../assets/models/fox.glb")?;

    let glb = gltf::Glb::from_slice(&bytes)?;

    // JSONをパース
    let json: serde_json::Value =
        serde_json::from_slice(&glb.json)?;

    // 読みやすく整形する
    let pretty_json = serde_json::to_string_pretty(&json)?;

    // コンソールに表示
    println!("{pretty_json}");

    // JSONファイルに保存
    fs::write(output_path, pretty_json)?;

    println!("Saved: {}", output_path.display());

    Ok(())
}

