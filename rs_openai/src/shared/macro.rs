use crate::apis::completions::Prompt;
use crate::apis::embeddings::EmbeddingInput;
use crate::apis::moderations::ModerationInput;
use crate::shared::types::Stop;

macro_rules! impl_default {
    ($to_typ:ty) => {
        impl Default for $to_typ {
            fn default() -> Self {
                <$to_typ>::String("".to_owned())
            }
        }
    };
}

macro_rules! impl_from {
    ($from_typ:ty, $to_typ:ty) => {
        impl From<$from_typ> for $to_typ {
            fn from(value: $from_typ) -> Self {
                <$to_typ>::String(value.into())
            }
        }

        impl From<Vec<$from_typ>> for $to_typ {
            fn from(value: Vec<$from_typ>) -> Self {
                <$to_typ>::ArrayOfString(value.iter().map(|v| v.to_string()).collect())
            }
        }

        impl From<&Vec<$from_typ>> for $to_typ {
            fn from(value: &Vec<$from_typ>) -> Self {
                <$to_typ>::ArrayOfString(value.iter().map(|v| v.to_string()).collect())
            }
        }

        impl<const N: usize> From<[$from_typ; N]> for $to_typ {
            fn from(value: [$from_typ; N]) -> Self {
                <$to_typ>::ArrayOfString(value.into_iter().map(|v| v.to_string()).collect())
            }
        }

        impl<const N: usize> From<&[$from_typ; N]> for $to_typ {
            fn from(value: &[$from_typ; N]) -> Self {
                <$to_typ>::ArrayOfString(value.into_iter().map(|v| v.to_string()).collect())
            }
        }
    };
}

macro_rules! impl_from_for_array_of_tokens {
    ($from_typ:ty, $to_typ:ty) => {
        impl<const N: usize> From<[$from_typ; N]> for $to_typ {
            fn from(value: [$from_typ; N]) -> Self {
                Self::ArrayOfTokens(value.to_vec())
            }
        }

        impl<const N: usize> From<&[$from_typ; N]> for $to_typ {
            fn from(value: &[$from_typ; N]) -> Self {
                Self::ArrayOfTokens(value.to_vec())
            }
        }

        impl From<Vec<$from_typ>> for $to_typ {
            fn from(value: Vec<$from_typ>) -> Self {
                Self::ArrayOfTokens(value)
            }
        }

        impl From<&Vec<$from_typ>> for $to_typ {
            fn from(value: &Vec<$from_typ>) -> Self {
                Self::ArrayOfTokens(value.clone())
            }
        }
    };
}

macro_rules! impl_from_for_array_of_token_arrays {
    ($from_typ:ty, $to_typ:ty) => {
        impl From<Vec<Vec<$from_typ>>> for $to_typ {
            fn from(value: Vec<Vec<$from_typ>>) -> Self {
                Self::ArrayOfTokenArrays(value)
            }
        }

        impl From<&Vec<Vec<$from_typ>>> for $to_typ {
            fn from(value: &Vec<Vec<$from_typ>>) -> Self {
                Self::ArrayOfTokenArrays(value.clone())
            }
        }

        impl<const M: usize, const N: usize> From<[[$from_typ; N]; M]> for $to_typ {
            fn from(value: [[$from_typ; N]; M]) -> Self {
                Self::ArrayOfTokenArrays(value.iter().map(|inner| inner.to_vec()).collect())
            }
        }

        impl<const M: usize, const N: usize> From<[&[$from_typ; N]; M]> for $to_typ {
            fn from(value: [&[$from_typ; N]; M]) -> Self {
                Self::ArrayOfTokenArrays(value.iter().map(|inner| inner.to_vec()).collect())
            }
        }

        impl<const M: usize, const N: usize> From<&[[$from_typ; N]; M]> for $to_typ {
            fn from(value: &[[$from_typ; N]; M]) -> Self {
                Self::ArrayOfTokenArrays(value.iter().map(|inner| inner.to_vec()).collect())
            }
        }

        impl<const M: usize, const N: usize> From<&[&[$from_typ; N]; M]> for $to_typ {
            fn from(value: &[&[$from_typ; N]; M]) -> Self {
                Self::ArrayOfTokenArrays(value.iter().map(|inner| inner.to_vec()).collect())
            }
        }

        impl<const N: usize> From<[Vec<$from_typ>; N]> for $to_typ {
            fn from(value: [Vec<$from_typ>; N]) -> Self {
                Self::ArrayOfTokenArrays(value.to_vec())
            }
        }

        impl<const N: usize> From<&[Vec<$from_typ>; N]> for $to_typ {
            fn from(value: &[Vec<$from_typ>; N]) -> Self {
                Self::ArrayOfTokenArrays(value.to_vec())
            }
        }

        impl<const N: usize> From<[&Vec<$from_typ>; N]> for $to_typ {
            fn from(value: [&Vec<$from_typ>; N]) -> Self {
                Self::ArrayOfTokenArrays(value.into_iter().map(|inner| inner.clone()).collect())
            }
        }

        impl<const N: usize> From<&[&Vec<$from_typ>; N]> for $to_typ {
            fn from(value: &[&Vec<$from_typ>; N]) -> Self {
                Self::ArrayOfTokenArrays(
                    value
                        .to_vec()
                        .into_iter()
                        .map(|inner| inner.clone())
                        .collect(),
                )
            }
        }

        impl<const N: usize> From<Vec<[$from_typ; N]>> for $to_typ {
            fn from(value: Vec<[$from_typ; N]>) -> Self {
                Self::ArrayOfTokenArrays(value.into_iter().map(|inner| inner.to_vec()).collect())
            }
        }

        impl<const N: usize> From<&Vec<[$from_typ; N]>> for $to_typ {
            fn from(value: &Vec<[$from_typ; N]>) -> Self {
                Self::ArrayOfTokenArrays(value.into_iter().map(|inner| inner.to_vec()).collect())
            }
        }

        impl<const N: usize> From<Vec<&[$from_typ; N]>> for $to_typ {
            fn from(value: Vec<&[$from_typ; N]>) -> Self {
                Self::ArrayOfTokenArrays(value.into_iter().map(|inner| inner.to_vec()).collect())
            }
        }

        impl<const N: usize> From<&Vec<&[$from_typ; N]>> for $to_typ {
            fn from(value: &Vec<&[$from_typ; N]>) -> Self {
                Self::ArrayOfTokenArrays(value.into_iter().map(|inner| inner.to_vec()).collect())
            }
        }
    };
}

impl_from!(String, ModerationInput);
impl_from!(&String, ModerationInput);
impl_from!(&str, ModerationInput);

impl_from!(String, EmbeddingInput);
impl_from!(&String, EmbeddingInput);
impl_from!(&str, EmbeddingInput);

impl_from!(String, Stop);
impl_from!(&String, Stop);
impl_from!(&str, Stop);

impl_from!(String, Prompt);
impl_from!(&String, Prompt);
impl_from!(&str, Prompt);

impl_default!(ModerationInput);
impl_default!(EmbeddingInput);

impl_from_for_array_of_tokens!(u16, Prompt);
impl_from_for_array_of_tokens!(u16, EmbeddingInput);

impl_from_for_array_of_token_arrays!(u16, Prompt);
impl_from_for_array_of_token_arrays!(u16, EmbeddingInput);
