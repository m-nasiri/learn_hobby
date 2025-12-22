use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewError {
    Unknown,
}

impl ViewError {
    #[must_use]
    pub fn message() -> &'static str {
        "Something went wrong. Please try again."
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
    resource: Resource<Result<T, ViewError>>,
) -> ViewState<T> {
    match resource.state().cloned() {
        UseResourceState::Pending => ViewState::Loading,
        UseResourceState::Ready => match resource.value().read().as_ref() {
            Some(Ok(data)) => ViewState::Ready(data.clone()),
            Some(Err(err)) => ViewState::Error(*err),
            None => ViewState::Error(ViewError::Unknown),
        },
        UseResourceState::Paused | UseResourceState::Stopped => ViewState::Idle,
    }
}
