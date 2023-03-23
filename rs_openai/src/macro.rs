use crate::chat::Stop;
use crate::moderations::ModerationInput;
use crate::completions::Prompt;

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

macro_rules! impl_default {
    ($to_typ:ty) => {
        impl Default for $to_typ {
            fn default() -> Self {
                <$to_typ>::String("".to_owned())
            }
        }
    };
}

impl_from!(String, ModerationInput);
impl_from!(&String, ModerationInput);
impl_from!(&str, ModerationInput);

impl_from!(String, Stop);
impl_from!(&String, Stop);
impl_from!(&str, Stop);

impl_from!(String, Prompt);
impl_from!(&String, Prompt);
impl_from!(&str, Prompt);

impl_default!(ModerationInput);
impl_default!(Stop);
impl_default!(Prompt);