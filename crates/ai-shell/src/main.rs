mod ai_bar;
mod components;
mod config;
mod daemon_config;
mod menubar;
mod onboarding;
mod panel;
mod settings;
mod theme;
mod wallpaper;

use anyhow::Result;
use eframe::egui;

use config::AppConfig;
use theme::Theme;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("AI-OS")
            .with_fullscreen(true)
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "AI-OS",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    ).map_err(|e| anyhow::anyhow!("{e}"))
}

struct App {
    config:          AppConfig,
    theme:           Theme,
    ai_bar:          ai_bar::AiBar,
    panel:           panel::Panel,
    menubar:         menubar::MenuBar,
    settings_open:   bool,
    settings_ui:     settings::SettingsUi,
    onboarding:      onboarding::Onboarding,
    show_onboarding: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        let config = AppConfig::load();
        let theme  = Theme::from_config(&config.appearance);
        theme.apply(&cc.egui_ctx);

        let show_onboarding = onboarding::Onboarding::needs_show(false);

        Self {
            ai_bar:          ai_bar::AiBar::new(),
            panel:           panel::Panel::new(&config),
            menubar:         menubar::MenuBar::new(),
            settings_open:   false,
            settings_ui:     settings::SettingsUi::new(config.clone()),
            onboarding:      onboarding::Onboarding::new(),
            show_onboarding,
            config,
            theme,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // sfondo / wallpaper
        wallpaper::draw(ctx, &self.config.appearance);

        // barra menù in alto (stile macOS)
        self.menubar.show(ctx, &self.config);

        // pannello laterale con le icone
        self.panel.show(ctx, &mut self.config, &mut self.settings_open);

        // barra AI in basso
        self.ai_bar.show(ctx);

        // Onboarding — mostrato solo quando api_key non configurata
        if self.show_onboarding {
            match self.onboarding.show(ctx) {
                onboarding::OnboardingAction::OpenSettings => {
                    self.show_onboarding = false;
                    self.settings_open   = true;
                    self.settings_ui.jump_to(3); // sezione "AI & Budget"
                }
                onboarding::OnboardingAction::Dismiss => {
                    self.show_onboarding = false;
                }
                onboarding::OnboardingAction::None => {}
            }
        }

        // pannello impostazioni (modale)
        if self.settings_open {
            if let Some(new_cfg) = self.settings_ui.show(ctx) {
                self.config = new_cfg.clone();
                self.theme  = Theme::from_config(&new_cfg.appearance);
                self.theme.apply(ctx);
                self.panel  = panel::Panel::new(&self.config);
                new_cfg.save();
                self.settings_open = false;
                // Se la API key è stata appena salvata, nascondi definitivamente l'onboarding
                self.show_onboarding = onboarding::Onboarding::needs_show(false);
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
