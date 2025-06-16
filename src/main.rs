use clap::Parser;
use csv::{ReaderBuilder, WriterBuilder};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, error::Error, fs::File, path::PathBuf, thread::sleep, time::Duration,
};

const API_URL: &str = "https://api-free.deepl.com/v2/translate";

#[derive(Debug, Deserialize, Serialize)]
struct Record {
    code: String,
    libellé: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    libellé_en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    libellé_de: Option<String>,
}

#[derive(Parser, Debug)]
#[command(
    name = "CSV Translator",
    about = "Traduit un fichier CSV FR -> EN/DE avec DeepL"
)]
struct Args {
    /// Fichier CSV d'entrée
    input: PathBuf,

    /// Séparateur de champs (par défaut: ',')
    #[arg(short = 's', long = "separator", default_value = ";")]
    separator: char,

    /// Clé API DeepL
    #[arg(short = 'k', long = "api-key")]
    api_key: String,

    /// Limite de lignes à traduire (optionnel)
    #[arg(short = 'n', long = "limit", default_value = "60000")]
    limit: Option<u16>,
}

fn translate(
    text: &str,
    target_lang: &str,
    client: &Client,
    api_key: &str,
) -> Result<String, Box<dyn Error>> {
    let mut params = HashMap::new();
    params.insert("auth_key", api_key);
    params.insert("text", text);
    params.insert("source_lang", "FR");
    params.insert("target_lang", target_lang);

    let resp = client
        .post(API_URL)
        .form(&params)
        .send()?
        .json::<serde_json::Value>()?;
    let translated = resp["translations"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok(translated)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let client = Client::new();
    let input_file = File::open(&args.input)?;
    let rdr = ReaderBuilder::new()
        .delimiter(args.separator as u8)
        .from_reader(input_file);

    let output_path = args
        .input
        .with_extension("")
        .with_extension("translated.csv");
    let output_file = File::create(&output_path)?;
    let mut wtr = WriterBuilder::new()
        .delimiter(args.separator as u8)
        .from_writer(output_file);

    let mut translated_count = 0;
    for result in rdr.into_deserialize::<Record>() {
        let mut record = result?;

        if Some(translated_count)>=args.limit{
            break;
        }
        /* 
        if let Some(limit) = args.limit {
            if translated_count >= limit {
                // Écrire la ligne non traduite telle quelle
                wtr.serialize(&record)?;
                continue;
            }
        }*/

        record.libellé_en = Some(translate(&record.libellé, "EN", &client, &args.api_key)?);
        sleep(Duration::from_millis(500));
        record.libellé_de = Some(translate(&record.libellé, "DE", &client, &args.api_key)?);
        sleep(Duration::from_millis(500));
        println!(
            "🔁 Traduction: {} en -> {:?} - de -> {:?}",
            record.libellé, record.libellé_en, record.libellé_de
        );

        translated_count += 1;
        wtr.serialize(&record)?;
    }

    wtr.flush()?;
    println!("✅ Fichier traduit : {}", output_path.display());

    Ok(())
}
