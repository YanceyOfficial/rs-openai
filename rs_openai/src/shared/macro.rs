use crate::interfaces::completions::Prompt;
use crate::interfaces::embeddings::EmbeddingInput;
use crate::interfaces::moderations::ModerationInput;
use crate::shared::types::Stop;

// NOTE ON DESIGN: the "obvious" simplification here is a single set of blanket
// impls generic over `S: Into<String>` (one each for the scalar, `Vec<S>`, and
// `[S; N]` shapes). That doesn't compile: Rust's coherence checker rejects
// `impl<S: Into<String>> From<S> for X` alongside `impl<S: Into<String>> From<Vec<S>> for X`
// (E0119) because it can't rule out some future `Vec<S>: Into<String>` impl
// making both apply to the same type, and for the same reason a blanket
// `From<Vec<S>>` conflicts with the concrete `From<Vec<u16>>` token-array impl
// below. So instead we keep this to exactly the three concrete "string-like"
// source types (`String`, `&str`, `&String`) that the old macro ever supported
// — nothing in this codebase constructs these from any other `Into<String>`
// type — which sidesteps the coherence issue entirely since none of these
// concrete types can ever unify with `Vec<u16>` or `Vec<Vec<u16>>`.
macro_rules! impl_string_like_from {
    ($to_typ:ty, $from_typ:ty, $string_variant:ident, $array_variant:ident) => {
        impl From<$from_typ> for $to_typ {
            fn from(value: $from_typ) -> Self {
                Self::$string_variant(value.into())
            }
        }

        impl From<Vec<$from_typ>> for $to_typ {
            fn from(value: Vec<$from_typ>) -> Self {
                Self::$array_variant(value.into_iter().map(Into::into).collect())
            }
        }

        impl<const N: usize> From<[$from_typ; N]> for $to_typ {
            fn from(value: [$from_typ; N]) -> Self {
                Self::$array_variant(value.into_iter().map(Into::into).collect())
            }
        }
    };
}

macro_rules! impl_string_like_from_all {
    ($to_typ:ty, $string_variant:ident, $array_variant:ident) => {
        impl_string_like_from!($to_typ, String, $string_variant, $array_variant);
        impl_string_like_from!($to_typ, &str, $string_variant, $array_variant);
        impl_string_like_from!($to_typ, &String, $string_variant, $array_variant);
    };
}

impl_string_like_from_all!(Stop, String, ArrayOfString);
impl_string_like_from_all!(Prompt, String, ArrayOfString);
impl_string_like_from_all!(EmbeddingInput, String, ArrayOfString);
impl_string_like_from_all!(ModerationInput, String, ArrayOfString);

impl Default for ModerationInput {
    fn default() -> Self {
        ModerationInput::String(String::new())
    }
}

impl Default for EmbeddingInput {
    fn default() -> Self {
        EmbeddingInput::String(String::new())
    }
}

macro_rules! impl_token_array_from {
    ($to_typ:ty) => {
        impl From<Vec<u16>> for $to_typ {
            fn from(value: Vec<u16>) -> Self {
                Self::ArrayOfTokens(value)
            }
        }

        impl From<Vec<Vec<u16>>> for $to_typ {
            fn from(value: Vec<Vec<u16>>) -> Self {
                Self::ArrayOfTokenArrays(value)
            }
        }
    };
}

impl_token_array_from!(Prompt);
impl_token_array_from!(EmbeddingInput);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_from_str_serializes_as_string() {
        let stop: Stop = "STOP".into();
        assert_eq!(
            serde_json::to_value(&stop).unwrap(),
            serde_json::json!("STOP")
        );
    }

    #[test]
    fn stop_from_array_serializes_as_array() {
        let stop: Stop = ["STOP1", "STOP2"].into();
        assert_eq!(
            serde_json::to_value(&stop).unwrap(),
            serde_json::json!(["STOP1", "STOP2"])
        );
    }

    #[test]
    fn prompt_from_token_vec_serializes_as_array_of_numbers() {
        let prompt: Prompt = vec![1_u16, 2, 3].into();
        assert_eq!(
            serde_json::to_value(&prompt).unwrap(),
            serde_json::json!([1, 2, 3])
        );
    }

    #[test]
    fn prompt_from_token_array_arrays_serializes_correctly() {
        let prompt: Prompt = vec![vec![1_u16, 2], vec![3_u16, 4]].into();
        assert_eq!(
            serde_json::to_value(&prompt).unwrap(),
            serde_json::json!([[1, 2], [3, 4]])
        );
    }

    #[test]
    fn embedding_input_from_array_of_str_serializes_as_array() {
        let input: EmbeddingInput = ["hello", "world"].into();
        assert_eq!(
            serde_json::to_value(&input).unwrap(),
            serde_json::json!(["hello", "world"])
        );
    }

    #[test]
    fn moderation_input_default_serializes_as_empty_string() {
        assert_eq!(
            serde_json::to_value(ModerationInput::default()).unwrap(),
            serde_json::json!("")
        );
    }
}
