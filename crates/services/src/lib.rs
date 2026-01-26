#![forbid(unsafe_code)]

pub mod error;
pub mod ai;
pub mod app_services;
pub mod app_settings_service;
pub mod card_service;
pub mod deck_service;
pub mod review_service;
pub mod sessions;
pub mod writing_tools_service;

pub use learn_core::Clock;
pub use sessions as session;

pub use error::{
    AiUsageError, AppSettingsServiceError, CardServiceError, DeckServiceError, ReviewServiceError,
    SessionError, WritingToolsError,
};
pub use error::AppServicesError;
pub use app_settings_service::AppSettingsService;
pub use app_services::AppServices;
pub use card_service::{
    CardListFilter, CardListSort, CardService, DeckPracticeStats, DeckPracticeStatsRow,
    TagPracticeStats,
};
pub use deck_service::DeckService;
pub use review_service::{PersistedReview, ReviewResult, ReviewService};
pub use writing_tools_service::{WritingToolsConfig, WritingToolsOutput, WritingToolsService};
pub use ai::{AiUsageHandle, AiUsageService};

pub use sessions::{
    ReviewGradePreview, SessionAnswerResult, SessionLoopService, SessionReview,
    SessionReviewPreview, SessionService, SessionSummaryDeckItem, SessionSummaryId,
    SessionSummaryListItem, SessionSummaryService,
};
