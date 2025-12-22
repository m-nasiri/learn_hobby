use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ViewError {
    /// Generic fallback when the UI cannot classify the failure.
    Unknown,

    /// A transient failure (e.g., network or IO) where retry is likely to help.
    Transient,
}

impl ViewError {
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            ViewError::Unknown => "Something went wrong. Please try again.",
            ViewError::Transient => "Temporary problem. Please try again in a moment.",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewState<T> {
    Idle,
    Loading,
    Ready(T),
    Error(ViewError),
}

#[must_use]
pub fn view_state_from_resource<T: Clone>(
    resource: &Resource<Result<T, ViewError>>,
) -> ViewState<T> {
    match *resource.state().read() {
        UseResourceState::Pending => ViewState::Loading,
        UseResourceState::Ready => {
            let value_sig = resource.value();
            let value = value_sig.read();
            match value.as_ref() {
                Some(Ok(data)) => ViewState::Ready(data.clone()),
                Some(Err(err)) => ViewState::Error(*err),
                None => ViewState::Error(ViewError::Unknown),
            }
        }
        UseResourceState::Paused | UseResourceState::Stopped => ViewState::Idle,
    }
}
