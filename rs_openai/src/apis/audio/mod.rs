//! Learn how to turn audio into text or text into audio. Related guide: [Speech to text](https://platform.openai.com/docs/guides/speech-to-text)

pub mod voice_consents;
pub mod voices;

use crate::client::OpenAI;
use crate::interfaces::audio;
pub use crate::interfaces::audio::*;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use crate::shared::utils::generate_random_string;
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
        let format = req.response_format.clone().unwrap_or_default();
        let random_filename = generate_random_string(16);
        self.openai
            .post_with_file_response("/audio/speech", req, &format!("{random_filename}.{format}"))
            .await
    }

    /// Transcribes audio into the input language, response is `application/json`.
    pub async fn create_transcription(
        &self,
        req: &audio::CreateTranscriptionRequest,
    ) -> OpenAIResponse<audio::SttResponse> {
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
    ) -> OpenAIResponse<audio::SttResponse> {
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

    /// Scoped accessor for voice-cloning consent records. Not yet implemented — see
    /// docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.
    pub fn voice_consents(&self) -> voice_consents::AudioVoiceConsents<'a> {
        voice_consents::AudioVoiceConsents::new(self.openai)
    }

    /// Scoped accessor for a specific voice-cloning consent record. Not yet implemented.
    pub fn voice_consent(&self, id: impl Into<String>) -> voice_consents::AudioVoiceConsent<'a> {
        voice_consents::AudioVoiceConsent::new(self.openai, id.into())
    }

    /// Scoped accessor for custom voice registration. Not yet implemented.
    pub fn voices(&self) -> voices::AudioVoices<'a> {
        voices::AudioVoices::new(self.openai)
    }

    fn create_transcription_form(&self, req: &audio::CreateTranscriptionRequest) -> Form {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

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
            form = form.text("language", language.to_string());
        }

        if let Some(timestamp_granularities) = req.timestamp_granularities.clone() {
            for (index, value) in timestamp_granularities.iter().enumerate() {
                form = form.text(
                    format!("timestamp_granularities[{}]", index),
                    value.to_string(),
                );
            }
        }

        form
    }

    fn create_translation_form(&self, req: &audio::CreateTranslationRequest) -> Form {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

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

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::audio::{CreateTranscriptionRequest, Language, SttModel};
    use crate::shared::types::File;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct ContainsInBody(&'static str);

    impl wiremock::Match for ContainsInBody {
        fn matches(&self, request: &Request) -> bool {
            String::from_utf8_lossy(&request.body).contains(self.0)
        }
    }

    #[tokio::test]
    async fn create_transcription_sends_correctly_spelled_language_field() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .and(ContainsInBody("name=\"language\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "text": "hello"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateTranscriptionRequest::builder()
            .file(File {
                buffer: vec![1, 2, 3],
                filename: "audio.mp4".into(),
            })
            .model(SttModel::Whisper1)
            .language(Language::Japanese)
            .build();

        let res = client.audio().create_transcription(&req).await.unwrap();
        assert_eq!(res.text, "hello");
    }
}
