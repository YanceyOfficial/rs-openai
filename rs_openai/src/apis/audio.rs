//! Learn how to turn audio into text.
//!
//! Related guide: [Speech to text](https://platform.openai.com/docs/guides/speech-to-text)

use crate::shared::response_wrapper::OpenAIError;
use crate::shared::types::FileMeta;
use crate::{OpenAI, OpenAIResponse};
use derive_builder::Builder;
use reqwest::multipart::Form;
use serde::{Deserialize, Serialize};

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
    #[default]
    #[strum(serialize = "en")]
    English,
    #[strum(serialize = "zh")]
    Chinese,
    #[strum(serialize = "de")]
    German,
    #[strum(serialize = "es")]
    Spanish,
    #[strum(serialize = "ru")]
    Russian,
    #[strum(serialize = "ko")]
    Korean,
    #[strum(serialize = "fr")]
    French,
    #[strum(serialize = "ja")]
    Japanese,
    #[strum(serialize = "pt")]
    Portuguese,
    #[strum(serialize = "tr")]
    Turkish,
    #[strum(serialize = "pl")]
    Polish,
    #[strum(serialize = "ca")]
    Catalan,
    #[strum(serialize = "nl")]
    Dutch,
    #[strum(serialize = "ar")]
    Arabic,
    #[strum(serialize = "sv")]
    Swedish,
    #[strum(serialize = "it")]
    Italian,
    #[strum(serialize = "id")]
    Indonesian,
    #[strum(serialize = "hi")]
    Hindi,
    #[strum(serialize = "fi")]
    Finnish,
    #[strum(serialize = "vi")]
    Vietnamese,
    #[strum(serialize = "he")]
    Hebrew,
    #[strum(serialize = "uk")]
    Ukrainian,
    #[strum(serialize = "el")]
    Greek,
    #[strum(serialize = "ms")]
    Malay,
    #[strum(serialize = "cs")]
    Czech,
    #[strum(serialize = "ro")]
    Romanian,
    #[strum(serialize = "da")]
    Danish,
    #[strum(serialize = "hu")]
    Hungarian,
    #[strum(serialize = "ta")]
    Tamil,
    #[strum(serialize = "no")]
    Norwegian,
    #[strum(serialize = "th")]
    Thai,
    #[strum(serialize = "ur")]
    Urdu,
    #[strum(serialize = "hr")]
    Croatian,
    #[strum(serialize = "bg")]
    Bulgarian,
    #[strum(serialize = "lt")]
    Lithuanian,
    #[strum(serialize = "la")]
    Latin,
    #[strum(serialize = "mi")]
    Maori,
    #[strum(serialize = "ml")]
    Malayalam,
    #[strum(serialize = "cy")]
    Welsh,
    #[strum(serialize = "sk")]
    Slovak,
    #[strum(serialize = "te")]
    Telugu,
    #[strum(serialize = "fa")]
    Persian,
    #[strum(serialize = "lv")]
    Latvian,
    #[strum(serialize = "bn")]
    Bengali,
    #[strum(serialize = "sr")]
    Serbian,
    #[strum(serialize = "az")]
    Azerbaijani,
    #[strum(serialize = "sl")]
    Slovenian,
    #[strum(serialize = "kn")]
    Kannada,
    #[strum(serialize = "et")]
    Estonian,
    #[strum(serialize = "mk")]
    Macedonian,
    #[strum(serialize = "br")]
    Breton,
    #[strum(serialize = "eu")]
    Basque,
    #[strum(serialize = "is")]
    Icelandic,
    #[strum(serialize = "hy")]
    Armenian,
    #[strum(serialize = "ne")]
    Nepali,
    #[strum(serialize = "mn")]
    Mongolian,
    #[strum(serialize = "bs")]
    Bosnian,
    #[strum(serialize = "kk")]
    Kazakh,
    #[strum(serialize = "sq")]
    Albanian,
    #[strum(serialize = "sw")]
    Swahili,
    #[strum(serialize = "gl")]
    Galician,
    #[strum(serialize = "mr")]
    Marathi,
    #[strum(serialize = "pa")]
    Punjabi,
    #[strum(serialize = "si")]
    Sinhala,
    #[strum(serialize = "km")]
    Khmer,
    #[strum(serialize = "sn")]
    Shona,
    #[strum(serialize = "yo")]
    Yoruba,
    #[strum(serialize = "so")]
    Somali,
    #[strum(serialize = "af")]
    Afrikaans,
    #[strum(serialize = "oc")]
    Occitan,
    #[strum(serialize = "ka")]
    Georgian,
    #[strum(serialize = "be")]
    Belarusian,
    #[strum(serialize = "tg")]
    Tajik,
    #[strum(serialize = "sd")]
    Sindhi,
    #[strum(serialize = "gu")]
    Gujarati,
    #[strum(serialize = "am")]
    Amharic,
    #[strum(serialize = "yi")]
    Yiddish,
    #[strum(serialize = "lo")]
    Lao,
    #[strum(serialize = "uz")]
    Uzbek,
    #[strum(serialize = "fo")]
    Faroese,
    #[strum(serialize = "ht")]
    HaitianCreole,
    #[strum(serialize = "ps")]
    Pashto,
    #[strum(serialize = "tk")]
    Turkmen,
    #[strum(serialize = "nn")]
    Nynorsk,
    #[strum(serialize = "mt")]
    Maltese,
    #[strum(serialize = "sa")]
    Sanskrit,
    #[strum(serialize = "lb")]
    Luxembourgish,
    #[strum(serialize = "my")]
    Myanmar,
    #[strum(serialize = "bo")]
    Tibetan,
    #[strum(serialize = "tl")]
    Tagalog,
    #[strum(serialize = "mg")]
    Malagasy,
    #[strum(serialize = "as")]
    Assamese,
    #[strum(serialize = "tt")]
    Tatar,
    #[strum(serialize = "haw")]
    Hawaiian,
    #[strum(serialize = "ln")]
    Lingala,
    #[strum(serialize = "ha")]
    Hausa,
    #[strum(serialize = "ba")]
    Bashkir,
    #[strum(serialize = "jw")]
    Javanese,
    #[strum(serialize = "su")]
    Sundanese,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum AudioModel {
    #[default]
    #[strum(serialize = "whisper-1")]
    Whisper1,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateTranscriptionRequestBuilder")]
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
#[builder(name = "CreateTranslationRequestBuilder")]
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

#[derive(Debug, Deserialize, Clone)]
pub struct VerboseJsonForAudioResponse {
    pub task: Option<String>,
    pub language: Option<String>,
    pub duration: Option<f32>,
    pub segments: Option<Vec<Segment>>,
    pub text: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Segment {
    pub id: u32,
    pub seek: u32,
    pub start: f32,
    pub end: f32,
    pub text: String,
    pub tokens: Vec<u32>,
    pub temperature: f32,
    pub avg_logprob: f32,
    pub compression_ratio: f32,
    pub no_speech_prob: f32,
}

pub struct Audio<'a> {
    openai: &'a OpenAI,
}

impl<'a> Audio<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Transcribes audio into the input language, response is `application/json`.
    pub async fn create_transcription(
        &self,
        req: &CreateTranscriptionRequest,
    ) -> OpenAIResponse<VerboseJsonForAudioResponse> {
        if !self.is_json_type(req.response_format.clone()) {
            return Err(OpenAIError::InvalidArgument(
    "When `response_format` is set to `ResponseFormat::Text` or `ResponseFormat::Vtt or `ResponseFormat::Srt`, use Audio::create_transcription_with_text_response".into(),
));
        }

        let form = self.create_transcription_form(req);
        self.openai.post_form("/audio/transcriptions", form).await
    }

    /// Translates audio into English, response is `application/json`.
    pub async fn create_translation(
        &self,
        req: &CreateTranslationRequest,
    ) -> OpenAIResponse<VerboseJsonForAudioResponse> {
        if !self.is_json_type(req.response_format.clone()) {
            return Err(OpenAIError::InvalidArgument(
        "When `response_format` is set to `ResponseFormat::Text` or `ResponseFormat::Vtt or `ResponseFormat::Srt`, use Audio::create_translation_with_text_response".into(),
    ));
        }

        let form = self.create_translation_form(req);
        self.openai.post_form("/audio/translations", form).await
    }

    /// Transcribes audio into the input language, response is `text/plain`.
    pub async fn create_transcription_with_text_response(
        &self,
        req: &CreateTranscriptionRequest,
    ) -> OpenAIResponse<String> {
        if self.is_json_type(req.response_format.clone()) {
            return Err(OpenAIError::InvalidArgument(
            "When response_format is `None` or `ResponseFormat::Json` or `ResponseFormat::VerboseJson`, use Audio::create_transcription".into(),
        ));
        }

        let form = self.create_transcription_form(req);
        self.openai
            .post_form_with_text_response("/audio/transcriptions", form)
            .await
    }

    /// Translates audio into English, response is `text/plain`.
    pub async fn create_translation_with_text_response(
        &self,
        req: &CreateTranslationRequest,
    ) -> OpenAIResponse<String> {
        if !self.is_json_type(req.response_format.clone()) {
            return Err(OpenAIError::InvalidArgument(
                "When response_format is `None` or `ResponseFormat::Json` or `ResponseFormat::VerboseJson`, use Audio::create_translation".into(),
            ));
        }

        let form = self.create_translation_form(req);
        self.openai
            .post_form_with_text_response("/audio/translations", form)
            .await
    }

    fn create_transcription_form(&self, req: &CreateTranscriptionRequest) -> Form {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        let mut form = Form::new()
            .part("file", file_part)
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

        if let Some(language) = req.language.clone() {
            form = form.text("laguage", language.to_string());
        }
        form
    }

    fn create_translation_form(&self, req: &CreateTranslationRequest) -> Form {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        let mut form = Form::new()
            .part("file", file_part)
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

        form
    }

    fn is_json_type(&self, format_type: Option<ResponseFormat>) -> bool {
        if format_type.is_none() {
            return true;
        }

        let format_type_display = format_type.unwrap().to_string();
        if format_type_display == ResponseFormat::Json.to_string()
            || format_type_display == ResponseFormat::VerboseJson.to_string()
        {
            return true;
        }

        return false;
    }
}
