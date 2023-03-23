//! Learn how to turn audio into text.
//!
//! Related guide: [Speech to text](https://platform.openai.com/docs/guides/speech-to-text)

use super::{OpenAI, Response};
use crate::error::OpenAIError;
use derive_builder::Builder;
use reqwest::multipart::Form;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Default, Debug, Clone)]
pub struct FileMeta {
    pub file_content: Vec<u8>,
    pub filename: String,
    pub content_type: String,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    #[strum(serialize = "json")]
    Json,
    #[strum(serialize = "text")]
    Text,
    #[strum(serialize = "srt")]
    Srt,
    #[strum(serialize = "verbose_json")]
    VerboseJson,
    #[strum(serialize = "vtt")]
    Vtt,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum Language {
    #[strum(serialize = "afrikaans")]
    Afrikaans,
    #[strum(serialize = "arabic")]
    Arabic,
    #[strum(serialize = "armenian")]
    Armenian,
    #[strum(serialize = "azerbaijani")]
    Azerbaijani,
    #[strum(serialize = "belarusian")]
    Belarusian,
    #[strum(serialize = "bosnian")]
    Bosnian,
    #[strum(serialize = "bulgarian")]
    Bulgarian,
    #[strum(serialize = "catalan")]
    Catalan,
    #[strum(serialize = "chinese")]
    Chinese,
    #[strum(serialize = "croatian")]
    Croatian,
    #[strum(serialize = "czech")]
    Czech,
    #[strum(serialize = "danish")]
    Danish,
    #[strum(serialize = "dutch")]
    Dutch,
    #[default]
    #[strum(serialize = "english")]
    English,
    #[strum(serialize = "estonian")]
    Estonian,
    #[strum(serialize = "finnish")]
    Finnish,
    #[strum(serialize = "french")]
    French,
    #[strum(serialize = "galician")]
    Galician,
    #[strum(serialize = "german")]
    German,
    #[strum(serialize = "greek")]
    Greek,
    #[strum(serialize = "hebrew")]
    Hebrew,
    #[strum(serialize = "hindi")]
    Hindi,
    #[strum(serialize = "hungarian")]
    Hungarian,
    #[strum(serialize = "icelandic")]
    Icelandic,
    #[strum(serialize = "indonesian")]
    Indonesian,
    #[strum(serialize = "italian")]
    Italian,
    #[strum(serialize = "japanese")]
    Japanese,
    #[strum(serialize = "kannada")]
    Kannada,
    #[strum(serialize = "kazakh")]
    Kazakh,
    #[strum(serialize = "korean")]
    Korean,
    #[strum(serialize = "latvian")]
    Latvian,
    #[strum(serialize = "lithuanian")]
    Lithuanian,
    #[strum(serialize = "macedonian")]
    Macedonian,
    #[strum(serialize = "malay")]
    Malay,
    #[strum(serialize = "marathi")]
    Marathi,
    #[strum(serialize = "maori")]
    Maori,
    #[strum(serialize = "nepali")]
    Nepali,
    #[strum(serialize = "norwegian")]
    Norwegian,
    #[strum(serialize = "persian")]
    Persian,
    #[strum(serialize = "polish")]
    Polish,
    #[strum(serialize = "portuguese")]
    Portuguese,
    #[strum(serialize = "romanian")]
    Romanian,
    #[strum(serialize = "russian")]
    Russian,
    #[strum(serialize = "serbian")]
    Serbian,
    #[strum(serialize = "slovak")]
    Slovak,
    #[strum(serialize = "slovenian")]
    Slovenian,
    #[strum(serialize = "spanish")]
    Spanish,
    #[strum(serialize = "swahili")]
    Swahili,
    #[strum(serialize = "swedish")]
    Swedish,
    #[strum(serialize = "tagalog")]
    Tagalog,
    #[strum(serialize = "tamil")]
    Tamil,
    #[strum(serialize = "thai")]
    Thai,
    #[strum(serialize = "turkish")]
    Turkish,
    #[strum(serialize = "ukrainian")]
    Ukrainian,
    #[strum(serialize = "urdu")]
    Urdu,
    #[strum(serialize = "vietnamese")]
    Vietnamese,
    #[strum(serialize = "welsh")]
    Welsh,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum AudioModel {
    #[default]
    #[strum(serialize = "whisper-1")]
    Whisper1,
}

// #[derive(Builder)]
// pub struct FileMetaBuilder {
//     #[builder(default)]
//     file: Vec<u8>,
//     #[builder(default)]
//     filename: String,
//     #[builder(default)]
//     content_type: String,
// }

// impl FileMetaBuilder {
//     pub fn build(self) -> Result<FileMeta, Box<dyn std::error::Error>> {
//         Ok(FileMeta {
//             file: self.file,
//             filename: self.filename,
//             content_type: self.content_type,
//         })
//     }
// }

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateTranscriptionRequestArgs")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateTranscriptionRequest {
    /// The audio file to transcribe, in one of these formats: mp3, mp4, mpeg, mpga, m4a, wav, or webm.
    pub file: FileMeta,

    /// ID of the model to use. Only `whisper-1` is currently available.
    pub model: AudioModel,

    /// An optional text to guide the model's style or continue a previous audio segment.
    /// The [prompt](https://platform.openai.com/docs/guides/speech-to-text/prompting) should match the audio language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// The format of the transcript output, in one of these options: json, text, srt, verbose_json, or vtt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "json"

    /// The sampling temperature, between 0 and 1. Higher values like 0.8 will make the output more random,
    /// while lower values like 0.2 will make it more focused and deterministic.
    /// If set to 0, the model will use [log probability](https://en.wikipedia.org/wiki/Log_probability) to automatically increase the temperature until certain thresholds are hit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // min: 0, max: 1, default: 0

    /// The language of the input audio. Supplying the input language in [ISO-639-1](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes) format will improve accuracy and latency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateTranslationRequestArgs")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateTranslationRequest {
    /// The audio file to transcribe, in one of these formats: mp3, mp4, mpeg, mpga, m4a, wav, or webm.
    pub file: FileMeta,

    /// ID of the model to use. Only `whisper-1` is currently available.
    pub model: AudioModel,

    /// An optional text to guide the model's style or continue a previous audio segment.
    /// The [prompt](https://platform.openai.com/docs/guides/speech-to-text/prompting) should be in English.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// The format of the transcript output, in one of these options: json, text, srt, verbose_json, or vtt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: json

    /// The sampling temperature, between 0 and 1. Higher values like 0.8 will make the output more random,
    /// while lower values like 0.2 will make it more focused and deterministic.
    /// If set to 0, the model will use [log probability](https://en.wikipedia.org/wiki/Log_probability) to automatically increase the temperature until certain thresholds are hit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // min: 0, max: 1, default: 0
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioResponse {
    pub text: String,
}

pub struct Audio<'a> {
    openai: &'a OpenAI<'a>,
}

impl<'a> Audio<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Transcribes audio into the input language.
    #[tokio::main]
    pub async fn transcribe(&self, req: &CreateTranscriptionRequest) -> Response<AudioResponse> {
        let file_part = reqwest::multipart::Part::stream(req.file.file_content.clone())
            .file_name("file_name.mp4")
            .mime_str("application/octet-stream")
            .unwrap();

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", "whisper-1");

        if let Some(language) = req.language.clone() {
            form = form.text("laguage", language.to_string());
        }

        if let Some(prompt) = req.prompt.clone() {
            form = form.text("prompt", prompt);
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(temperature) = req.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        println!("{:?}", form);

        self.openai.post_form("/audio/transcriptions", form).await
    }

    /// Translates audio into English.
    #[tokio::main]
    pub async fn translate(&self, req: &CreateTranslationRequest) -> Response<AudioResponse> {
        let mut form = Form::new()
            .part(
                req.file.filename.clone(),
                reqwest::multipart::Part::bytes(req.file.file_content.clone())
                    .file_name(req.file.filename.clone())
                    .mime_str("application/octet-stream")?,
            )
            .text("model", req.model.to_string());

        if let Some(prompt) = req.prompt.clone() {
            form = form.text("prompt", prompt);
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(temperature) = req.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        self.openai.post_form("/audio/translations", form).await
    }
}
