//! Gestionnaire de mises à jour et de pilotes au look Ubuntu « Human » (2008).
//!
//!   gestionnaire-maj               fenêtre principale
//!   gestionnaire-maj --installer   fenêtre + installation immédiate (depuis la bulle)
//!   gestionnaire-maj --pilotes     fenêtre ouverte sur l'onglet des pilotes
//!   gestionnaire-maj --mise-a-niveau  propose la nouvelle version d'Ubuntu
//!   gestionnaire-maj --veille      veille de fond lancée avec la session
//!   gestionnaire-maj --bulle [f]   bulle d'annonce (f = annonce préparée par la veille)
//!   gestionnaire-maj --lister      liste des mises à jour en JSON

mod app;
mod bulle;
mod commun;
mod theme;
mod veille;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let res = match args.first().map(String::as_str) {
        Some("--veille") => {
            veille::lancer();
            Ok(())
        }
        Some("--bulle") => bulle::lancer(args.get(1)),
        Some("--installer") => app::lancer(app::Mode::Installer),
        Some("--pilotes") => app::lancer(app::Mode::Pilotes),
        Some("--mise-a-niveau") => app::lancer(app::Mode::MiseANiveau),
        Some("--lister") => {
            let r = commun::chercher_tout(&commun::Reglages::charger());
            println!("{}", serde_json::to_string_pretty(&r).unwrap_or_default());
            Ok(())
        }
        Some("--lister-pilotes") => {
            println!("{}", serde_json::to_string_pretty(&commun::chercher_pilotes()).unwrap_or_default());
            Ok(())
        }
        Some("--version") => {
            println!("{} {}", commun::APP, commun::VERSION);
            Ok(())
        }
        Some("-h") | Some("--help") | Some("--aide") => {
            println!(
                "{} {}\n\n  gestionnaire-maj              fenêtre principale\n  gestionnaire-maj --installer  installe tout de suite les mises à jour\n  gestionnaire-maj --pilotes    ouvre l'onglet des pilotes\n  gestionnaire-maj --mise-a-niveau  propose la nouvelle version d'Ubuntu\n  gestionnaire-maj --veille     veille de fond (lancée au démarrage)\n  gestionnaire-maj --bulle      affiche la bulle s'il y a des mises à jour\n  gestionnaire-maj --lister     liste des mises à jour (JSON)",
                commun::NOM,
                commun::VERSION
            );
            Ok(())
        }
        _ => app::lancer(app::Mode::Normal),
    };
    if let Err(e) = res {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
