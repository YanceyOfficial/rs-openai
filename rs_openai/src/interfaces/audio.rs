use crate::shared::response_wrapper::OpenAIError;
use crate::shared::types::File;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum SttResponseFormat {
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
pub enum TtsResponseFormat {
    #[default]
    #[strum(serialize = "mp3")]
    Mp3,
    #[strum(serialize = "opus")]
    Opus,
    #[strum(serialize = "aac")]
    Aac,
    #[strum(serialize = "flac")]
    Flac,
    #[strum(serialize = "wav")]
    Wav,
    #[strum(serialize = "pcm")]
    Pcm,
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
pub enum Voice {
    #[default]
    #[strum(serialize = "alloy")]
    Alloy,
    #[strum(serialize = "echo")]
    Echo,
    #[strum(serialize = "fable")]
    Fable,
    #[strum(serialize = "onyx")]
    Onyx,
    #[strum(serialize = "nova")]
    Nova,
    #[strum(serialize = "shimmer")]
    Shimmer,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum SttModel {
    #[default]
    #[strum(serialize = "whisper-1")]
    Whisper1,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum TtsModel {
    #[default]
    #[strum(serialize = "tts-1")]
    Whisper1,
    #[strum(serialize = "tts-1-hd")]
    Whisper1Hd,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateSpeechRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateSpeechRequest {
    /// One of the available [TTS models](https://platform.openai.com/docs/models/tts): `tts-1` or `tts-1-hd`
    pub model: TtsModel,

    /// The text to generate audio for. The maximum length is 4096 characters.
    pub input: String,

    /// The voice to use when generating the audio. Supported voices are `alloy`, `echo`, `fable`, `onyx`, `nova`, and `shimmer`.
    /// Previews of the voices are available in the [Text to speech guide](https://platform.openai.com/docs/guides/text-to-speech/voice-options).
    pub voice: Voice,

    /// The format to audio in. Supported formats are `mp3`, `opus`, `aac`, `flac`, `wav`, and `pcm`.
    /// #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<TtsResponseFormat>, // default: mp3

    /// The speed of the generated audio. Select a value from `0.25` to `4.0`. `1.0` is the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>, // min: 0.25, max: 4.0, default: 1.0
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateTranscriptionRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateTranscriptionRequest {
    /// The audio file object (not file name) to transcribe, in one of these formats: flac, mp3, mp4, mpeg, mpga, m4a, ogg, wav, or webm.
    pub file: File,

    /// ID of the model to use. Only `whisper-1` (which is powered by our open source Whisper V2 model) is currently available.
    pub model: SttModel,

    /// The language of the input audio. Supplying the input language in [ISO-639-1](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes) format will improve accuracy and latency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,

    /// An optional text to guide the model's style or continue a previous audio segment.
    /// The [prompt](https://platform.openai.com/docs/guides/speech-to-text/prompting) should match the audio language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// The format of the transcript output, in one of these options: json, text, srt, verbose_json, or vtt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<SttResponseFormat>, // default: "json"

    /// The sampling temperature, between 0 and 1. Higher values like 0.8 will make the output more random,
    /// while lower values like 0.2 will make it more focused and deterministic.
    /// If set to 0, the model will use [log probability](https://en.wikipedia.org/wiki/Log_probability) to automatically increase the temperature until certain thresholds are hit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // min: 0, max: 1, default: 0

    /// The timestamp granularities to populate for this transcription.
    /// `response_format` must be set `verbose_json` to use timestamp granularities.
    /// Either or both of these options are supported: `word`, or `segment`. Note: There is no additional latency for segment timestamps, but generating word timestamps incurs additional latency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_granularities: Option<Vec<TimestampGranularity>>, // Defaults to segment
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateTranslationRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateTranslationRequest {
    /// The audio file object (not file name) to transcribe, in one of these formats: flac, mp3, mp4, mpeg, mpga, m4a, ogg, wav, or webm.
    pub file: File,

    /// ID of the model to use. Only `whisper-1` (which is powered by our open source Whisper V2 model) is currently available.
    pub model: SttModel,

    /// An optional text to guide the model's style or continue a previous audio segment.
    /// The [prompt](https://platform.openai.com/docs/guides/speech-to-text/prompting) should be in English.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// The format of the transcript output, in one of these options: json, text, srt, verbose_json, or vtt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<SttResponseFormat>, // Defaults to json

    /// The sampling temperature, between 0 and 1.
    /// Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic.
    /// If set to 0, the model will use [log probability](https://en.wikipedia.org/wiki/Log_probability) to automatically increase the temperature until certain thresholds are hit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // Defaults to 0
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum TimestampGranularity {
    #[default]
    #[strum(serialize = "segment")]
    Segment,
    #[strum(serialize = "word")]
    Word,
}

/// Represents a verbose json transcription response returned by model, based on the provided input.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SttResponse {
    /// The transcribed text.
    pub text: String,
    /// Always `transcribe`.
    pub task: Option<String>,
    /// The language of the input audio.
    pub language: Option<String>,
    /// The duration of the input audio.
    pub duration: Option<f32>,
    /// Segments of the transcribed text and their corresponding details.
    pub segments: Option<Vec<Segment>>,
    /// Extracted words and their corresponding timestamps.
    pub words: Option<Vec<Word>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Segment {
    /// Unique identifier of the segment.
    pub id: u32,
    /// Seek offset of the segment.
    pub seek: u32,
    /// Start time of the segment in seconds.
    pub start: f32,
    /// End time of the segment in seconds.
    pub end: f32,
    /// Text content of the segment.
    pub text: String,
    /// Array of token IDs for the text content.
    pub tokens: Vec<u32>,
    /// Temperature parameter used for generating the segment.
    pub temperature: f32,
    /// Average logprob of the segment. If the value is lower than -1, consider the logprobs failed.
    pub avg_logprob: f32,
    /// Compression ratio of the segment. If the value is greater than 2.4, consider the compression failed.
    pub compression_ratio: f32,
    /// Probability of no speech in the segment. If the value is higher than 1.0 and the `avg_logprob` is below -1, consider this segment silent.
    pub no_speech_prob: f32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Word {
    /// The text content of the word.
    pub word: String,
    /// Start time of the word in seconds.
    pub start: f32,
    /// End time of the word in seconds.
    pub end: f32,
}
