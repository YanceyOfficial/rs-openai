//! Learn how to turn audio into text.
//!
//! Related guide: [Speech to text](https://platform.openai.com/docs/guides/speech-to-text)

use crate::client::OpenAI;
use crate::interfaces::audio;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use reqwest::multipart::Form;

pub struct Audio<'a> {
    openai: &'a OpenAI,
}

impl<'a> Audio<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Generates audio from the input text.
    pub async fn create_speech(&self, req: &audio::CreateSpeechRequest) -> OpenAIResponse<()> {
        self.openai
            .post_with_file_response("/audio/speech", req, "")
            .await
    }

    /// Transcribes audio into the input language, response is `application/json`.
    pub async fn create_transcription(
        &self,
        req: &audio::CreateTranscriptionRequest,
    ) -> OpenAIResponse<audio::VerboseJsonForAudioResponse> {
        if !self.is_json_type(req.response_format.clone()) {
            return Err(OpenAIError::InvalidArgument(
    "When `response_format` is set to `SttResponseFormat::Text` or `SttResponseFormat::Vtt or `SttResponseFormat::Srt`, use Audio::create_transcription_with_text_response".into(),
));
        }

        let form = self.create_transcription_form(req);
        self.openai.post_form("/audio/transcriptions", form).await
    }

    /// Translates audio into English, response is `application/json`.
    pub async fn create_translation(
        &self,
        req: &audio::CreateTranslationRequest,
    ) -> OpenAIResponse<audio::VerboseJsonForAudioResponse> {
        if !self.is_json_type(req.response_format.clone()) {
            return Err(OpenAIError::InvalidArgument(
        "When `response_format` is set to `SttResponseFormat::Text` or `SttResponseFormat::Vtt or `SttResponseFormat::Srt`, use Audio::create_translation_with_text_response".into(),
    ));
        }

        let form = self.create_translation_form(req);
        self.openai.post_form("/audio/translations", form).await
    }

    /// Transcribes audio into the input language, response is `text/plain`.
    pub async fn create_transcription_with_text_response(
        &self,
        req: &audio::CreateTranscriptionRequest,
    ) -> OpenAIResponse<String> {
        if self.is_json_type(req.response_format.clone()) {
            return Err(OpenAIError::InvalidArgument(
            "When `response_format` is `None` or `SttResponseFormat::Json` or `SttResponseFormat::VerboseJson`, use Audio::create_transcription".into(),
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
        req: &audio::CreateTranslationRequest,
    ) -> OpenAIResponse<String> {
        if self.is_json_type(req.response_format.clone()) {
            return Err(OpenAIError::InvalidArgument(
                "When response_format is `None` or `SttResponseFormat::Json` or `SttResponseFormat::VerboseJson`, use Audio::create_translation".into(),
            ));
        }

        let form = self.create_translation_form(req);
        self.openai
            .post_form_with_text_response("/audio/translations", form)
            .await
    }

    fn create_transcription_form(&self, req: &audio::CreateTranscriptionRequest) -> Form {
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

    fn create_translation_form(&self, req: &audio::CreateTranslationRequest) -> Form {
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

    fn is_json_type(&self, format_type: Option<audio::SttResponseFormat>) -> bool {
        if format_type.is_none() {
            return true;
        }

        let format_type_display = format_type.unwrap().to_string();
        if format_type_display == audio::SttResponseFormat::Json.to_string()
            || format_type_display == audio::SttResponseFormat::VerboseJson.to_string()
        {
            return true;
        }

        false
    }
}
