/// Schermata di benvenuto mostrata al primo avvio quando l'API key non è configurata.
use egui::{Align2, Color32, Context, RichText, Window};

pub struct Onboarding {
    dismissed: bool,
}

/// Azione che l'onboarding chiede all'App di eseguire.
pub enum OnboardingAction {
    /// Nessuna azione.
    None,
    /// Apri le impostazioni direttamente sulla sezione "AI & Budget" (indice 3).
    OpenSettings,
    /// L'utente ha ignorato l'onboarding — non mostrarlo più in questa sessione.
    Dismiss,
}

impl Onboarding {
    pub fn new() -> Self {
        Self { dismissed: false }
    }

    /// Ritorna true se l'onboarding deve ancora essere mostrato.
    pub fn needs_show(dismissed: bool) -> bool {
        if dismissed { return false; }
        // Controlla se api_key è vuota nella config del daemon
        let text = dirs_next::config_dir()
            .unwrap_or_default()
            .join("ai-os")
            .join("config.toml");
        let content = std::fs::read_to_string(text).unwrap_or_default();
        // api_key vuota se la riga è 'api_key = ""' o 'api_key = ''
        let empty = content
            .lines()
            .find(|l| l.trim_start().starts_with("api_key"))
            .map(|l| {
                let val = l.splitn(2, '=').nth(1).unwrap_or("").trim().to_string();
                val == r#""""# || val == "''" || val.is_empty()
            })
            .unwrap_or(true);   // se il file non esiste → sicuramente non configurato
        empty
    }

    pub fn show(&mut self, ctx: &Context) -> OnboardingAction {
        if self.dismissed {
            return OnboardingAction::None;
        }

        let mut action = OnboardingAction::None;

        Window::new("Benvenuto in AI-OS")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                ui.label(
                    RichText::new("Prima configurazione")
                        .size(18.0)
                        .strong()
                        .color(Color32::from_rgb(126, 200, 227)),
                );

                ui.add_space(12.0);
                ui.label(
                    "AI-OS ha bisogno di una chiave API Anthropic per funzionare.\n\
                     La chiave non è ancora configurata.",
                );

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Istruzioni passo per passo
                ui.label(RichText::new("Come ottenere la chiave:").strong());
                ui.add_space(4.0);

                let steps = [
                    "1.  Vai su console.anthropic.com",
                    "2.  Accedi o crea un account",
                    "3.  API Keys → Create Key",
                    "4.  Copia la chiave (inizia con sk-ant-…)",
                    "5.  Incollala nelle impostazioni qui sotto",
                ];
                for step in &steps {
                    ui.label(RichText::new(*step).small().color(Color32::GRAY));
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button(
                        RichText::new("  Apri impostazioni  ").color(Color32::WHITE)
                    ).clicked() {
                        action = OnboardingAction::OpenSettings;
                        self.dismissed = true;
                    }

                    ui.add_space(8.0);

                    if ui.button("Ignora per ora").clicked() {
                        action = OnboardingAction::Dismiss;
                        self.dismissed = true;
                    }
                });

                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "Puoi riaprire questa schermata da: Impostazioni → AI & Budget"
                    )
                    .small()
                    .color(Color32::GRAY),
                );
            });

        action
    }
}
