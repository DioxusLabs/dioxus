use std::str::FromStr;

const MACOS_APP_CATEGORY_PREFIX: &str = "public.app-category.";

/// The possible app categories.
/// Corresponds to `LSApplicationCategoryType` on macOS and the GNOME desktop categories on Debian.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppCategory {
    Business,
    DeveloperTool,
    Education,
    Entertainment,
    Finance,
    Game,
    ActionGame,
    AdventureGame,
    ArcadeGame,
    BoardGame,
    CardGame,
    CasinoGame,
    DiceGame,
    EducationalGame,
    FamilyGame,
    KidsGame,
    MusicGame,
    PuzzleGame,
    RacingGame,
    RolePlayingGame,
    SimulationGame,
    SportsGame,
    StrategyGame,
    TriviaGame,
    WordGame,
    GraphicsAndDesign,
    HealthcareAndFitness,
    Lifestyle,
    Medical,
    Music,
    News,
    Photography,
    Productivity,
    Reference,
    SocialNetworking,
    Sports,
    Travel,
    Utility,
    Video,
    Weather,
}

impl FromStr for AppCategory {
    type Err = String;

    fn from_str(input: &str) -> Result<AppCategory, Self::Err> {
        let mut input = input.to_ascii_lowercase();
        if input.starts_with(MACOS_APP_CATEGORY_PREFIX) {
            input = input
                .split_at(MACOS_APP_CATEGORY_PREFIX.len())
                .1
                .to_string();
        }
        input = input.replace(' ', "");
        input = input.replace('-', "");

        for &(string, category) in CATEGORY_STRINGS.iter() {
            if input == string {
                return Ok(category);
            }
        }
        Err(format!("Unknown app category: {input}"))
    }
}

impl AppCategory {
    /// Map to closest set of Freedesktop categories.
    pub(crate) fn freedesktop_categories(self) -> &'static str {
        match &self {
            AppCategory::Business => "Office;",
            AppCategory::DeveloperTool => "Development;",
            AppCategory::Education => "Education;",
            AppCategory::Entertainment => "Network;",
            AppCategory::Finance => "Office;Finance;",
            AppCategory::Game => "Game;",
            AppCategory::ActionGame => "Game;ActionGame;",
            AppCategory::AdventureGame => "Game;AdventureGame;",
            AppCategory::ArcadeGame => "Game;ArcadeGame;",
            AppCategory::BoardGame => "Game;BoardGame;",
            AppCategory::CardGame => "Game;CardGame;",
            AppCategory::CasinoGame => "Game;",
            AppCategory::DiceGame => "Game;",
            AppCategory::EducationalGame => "Game;Education;",
            AppCategory::FamilyGame => "Game;",
            AppCategory::KidsGame => "Game;KidsGame;",
            AppCategory::MusicGame => "Game;",
            AppCategory::PuzzleGame => "Game;LogicGame;",
            AppCategory::RacingGame => "Game;",
            AppCategory::RolePlayingGame => "Game;RolePlaying;",
            AppCategory::SimulationGame => "Game;Simulation;",
            AppCategory::SportsGame => "Game;SportsGame;",
            AppCategory::StrategyGame => "Game;StrategyGame;",
            AppCategory::TriviaGame => "Game;",
            AppCategory::WordGame => "Game;",
            AppCategory::GraphicsAndDesign => "Graphics;",
            AppCategory::HealthcareAndFitness => "Science;",
            AppCategory::Lifestyle => "Education;",
            AppCategory::Medical => "Science;MedicalSoftware;",
            AppCategory::Music => "AudioVideo;Audio;Music;",
            AppCategory::News => "Network;News;",
            AppCategory::Photography => "Graphics;Photography;",
            AppCategory::Productivity => "Office;",
            AppCategory::Reference => "Education;",
            AppCategory::SocialNetworking => "Network;",
            AppCategory::Sports => "Education;Sports;",
            AppCategory::Travel => "Education;",
            AppCategory::Utility => "Utility;",
            AppCategory::Video => "AudioVideo;Video;",
            AppCategory::Weather => "Science;",
        }
    }

    /// Map to macOS LSApplicationCategoryType.
    pub(crate) fn macos_application_category_type(self) -> &'static str {
        match &self {
            AppCategory::Business => "public.app-category.business",
            AppCategory::DeveloperTool => "public.app-category.developer-tools",
            AppCategory::Education => "public.app-category.education",
            AppCategory::Entertainment => "public.app-category.entertainment",
            AppCategory::Finance => "public.app-category.finance",
            AppCategory::Game => "public.app-category.games",
            AppCategory::ActionGame => "public.app-category.action-games",
            AppCategory::AdventureGame => "public.app-category.adventure-games",
            AppCategory::ArcadeGame => "public.app-category.arcade-games",
            AppCategory::BoardGame => "public.app-category.board-games",
            AppCategory::CardGame => "public.app-category.card-games",
            AppCategory::CasinoGame => "public.app-category.casino-games",
            AppCategory::DiceGame => "public.app-category.dice-games",
            AppCategory::EducationalGame => "public.app-category.educational-games",
            AppCategory::FamilyGame => "public.app-category.family-games",
            AppCategory::KidsGame => "public.app-category.kids-games",
            AppCategory::MusicGame => "public.app-category.music-games",
            AppCategory::PuzzleGame => "public.app-category.puzzle-games",
            AppCategory::RacingGame => "public.app-category.racing-games",
            AppCategory::RolePlayingGame => "public.app-category.role-playing-games",
            AppCategory::SimulationGame => "public.app-category.simulation-games",
            AppCategory::SportsGame => "public.app-category.sports-games",
            AppCategory::StrategyGame => "public.app-category.strategy-games",
            AppCategory::TriviaGame => "public.app-category.trivia-games",
            AppCategory::WordGame => "public.app-category.word-games",
            AppCategory::GraphicsAndDesign => "public.app-category.graphics-design",
            AppCategory::HealthcareAndFitness => "public.app-category.healthcare-fitness",
            AppCategory::Lifestyle => "public.app-category.lifestyle",
            AppCategory::Medical => "public.app-category.medical",
            AppCategory::Music => "public.app-category.music",
            AppCategory::News => "public.app-category.news",
            AppCategory::Photography => "public.app-category.photography",
            AppCategory::Productivity => "public.app-category.productivity",
            AppCategory::Reference => "public.app-category.reference",
            AppCategory::SocialNetworking => "public.app-category.social-networking",
            AppCategory::Sports => "public.app-category.sports",
            AppCategory::Travel => "public.app-category.travel",
            AppCategory::Utility => "public.app-category.utilities",
            AppCategory::Video => "public.app-category.video",
            AppCategory::Weather => "public.app-category.weather",
        }
    }
}

const CATEGORY_STRINGS: &[(&str, AppCategory)] = &[
    ("actiongame", AppCategory::ActionGame),
    ("actiongames", AppCategory::ActionGame),
    ("adventuregame", AppCategory::AdventureGame),
    ("adventuregames", AppCategory::AdventureGame),
    ("arcadegame", AppCategory::ArcadeGame),
    ("arcadegames", AppCategory::ArcadeGame),
    ("boardgame", AppCategory::BoardGame),
    ("boardgames", AppCategory::BoardGame),
    ("business", AppCategory::Business),
    ("cardgame", AppCategory::CardGame),
    ("cardgames", AppCategory::CardGame),
    ("casinogame", AppCategory::CasinoGame),
    ("casinogames", AppCategory::CasinoGame),
    ("developer", AppCategory::DeveloperTool),
    ("developertool", AppCategory::DeveloperTool),
    ("developertools", AppCategory::DeveloperTool),
    ("development", AppCategory::DeveloperTool),
    ("dicegame", AppCategory::DiceGame),
    ("dicegames", AppCategory::DiceGame),
    ("education", AppCategory::Education),
    ("educationalgame", AppCategory::EducationalGame),
    ("educationalgames", AppCategory::EducationalGame),
    ("entertainment", AppCategory::Entertainment),
    ("familygame", AppCategory::FamilyGame),
    ("familygames", AppCategory::FamilyGame),
    ("finance", AppCategory::Finance),
    ("fitness", AppCategory::HealthcareAndFitness),
    ("game", AppCategory::Game),
    ("games", AppCategory::Game),
    ("graphicdesign", AppCategory::GraphicsAndDesign),
    ("graphicsanddesign", AppCategory::GraphicsAndDesign),
    ("graphicsdesign", AppCategory::GraphicsAndDesign),
    ("healthcareandfitness", AppCategory::HealthcareAndFitness),
    ("healthcarefitness", AppCategory::HealthcareAndFitness),
    ("kidsgame", AppCategory::KidsGame),
    ("kidsgames", AppCategory::KidsGame),
    ("lifestyle", AppCategory::Lifestyle),
    ("logicgame", AppCategory::PuzzleGame),
    ("medical", AppCategory::Medical),
    ("music", AppCategory::Music),
    ("musicgame", AppCategory::MusicGame),
    ("musicgames", AppCategory::MusicGame),
    ("news", AppCategory::News),
    ("photography", AppCategory::Photography),
    ("productivity", AppCategory::Productivity),
    ("puzzlegame", AppCategory::PuzzleGame),
    ("puzzlegames", AppCategory::PuzzleGame),
    ("racinggame", AppCategory::RacingGame),
    ("racinggames", AppCategory::RacingGame),
    ("reference", AppCategory::Reference),
    ("roleplaying", AppCategory::RolePlayingGame),
    ("roleplayinggame", AppCategory::RolePlayingGame),
    ("roleplayinggames", AppCategory::RolePlayingGame),
    ("rpg", AppCategory::RolePlayingGame),
    ("simulationgame", AppCategory::SimulationGame),
    ("simulationgames", AppCategory::SimulationGame),
    ("socialnetwork", AppCategory::SocialNetworking),
    ("socialnetworking", AppCategory::SocialNetworking),
    ("sports", AppCategory::Sports),
    ("sportsgame", AppCategory::SportsGame),
    ("sportsgames", AppCategory::SportsGame),
    ("strategygame", AppCategory::StrategyGame),
    ("strategygames", AppCategory::StrategyGame),
    ("travel", AppCategory::Travel),
    ("triviagame", AppCategory::TriviaGame),
    ("triviagames", AppCategory::TriviaGame),
    ("utilities", AppCategory::Utility),
    ("utility", AppCategory::Utility),
    ("video", AppCategory::Video),
    ("weather", AppCategory::Weather),
    ("wordgame", AppCategory::WordGame),
    ("wordgames", AppCategory::WordGame),
];
