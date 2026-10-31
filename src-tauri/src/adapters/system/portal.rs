//! Integração com o **host registry** do xdg-desktop-portal.
//!
//! Desde o xdg-desktop-portal 1.20/1.21, apps fora de sandbox precisam se
//! identificar antes de usar portais que exigem `app_id` (GlobalShortcuts):
//! `Registry.Register(app_id, {})` na **mesma conexão D-Bus** usada pelo portal,
//! com um id reverso-DNS que corresponda a um `.desktop` instalado.
//!
//! O AppImage/dev não instala um `.desktop` no sistema, então o app garante um
//! arquivo em `$XDG_DATA_HOME/applications` na primeira execução.

use std::collections::HashMap;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use zbus::zvariant::Value;

use crate::error::{GhostWhisperError, Result};

/// Id do aplicativo no portal (reverso-DNS).
///
/// Não usamos o identifier do Tauri (`com.ghostwhisper.desktop`) porque ele
/// terminaria em `.desktop` e o arquivo viraria `…desktop.desktop`; este id é
/// estável e o arquivo correspondente é `com.ghostwhisper.app.desktop`.
pub const APP_ID: &str = "com.ghostwhisper.app";

/// O Registry fica no objeto principal do portal.
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const REGISTRY_INTERFACE: &str = "org.freedesktop.host.portal.Registry";

/// Garante o `.desktop` que dá suporte ao `APP_ID`.
///
/// Mantém o arquivo atualizado (o app pode ser um AppImage ou o binário de dev).
/// O `Exec` aponta para o AppImage quando a variável `APPIMAGE` existe; no dev,
/// para o binário atual. `NoDisplay=true` evita duplicar o lançador no menu.
pub fn ensure_desktop_entry() -> Result<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .ok_or_else(|| {
            GhostWhisperError::Other("HOME/XDG_DATA_HOME indisponível para o portal".to_string())
        })?;
    let directory = data_home.join("applications");
    std::fs::create_dir_all(&directory).map_err(|error| {
        GhostWhisperError::Other(format!("não foi possível criar {directory:?}: {error}"))
    })?;
    let path = directory.join(format!("{APP_ID}.desktop"));

    let exec = std::env::var("APPIMAGE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "ghostwhisper".to_string());
    std::fs::write(&path, desktop_entry(&exec)).map_err(|error| {
        GhostWhisperError::Other(format!("não foi possível escrever {path:?}: {error}"))
    })?;
    Ok(path)
}

/// Abre uma conexão de sessão e faz o `Registry.Register`.
///
/// Em portais antigos o método pode não existir; nesse caso seguimos com a
/// conexão (o app id pode ser derivado do `.desktop`/scope) e deixamos o erro
/// no log. O registro só pode ser feito uma vez por conexão.
pub async fn connection() -> Result<zbus::Connection> {
    let connection = zbus::Connection::session().await.map_err(|error| {
        GhostWhisperError::Other(format!("conexão D-Bus do portal indisponível: {error}"))
    })?;
    if let Err(error) = register(&connection).await {
        tracing::warn!("host registry do portal não registrou {APP_ID}: {error}");
    }
    Ok(connection)
}

/// Chama `org.freedesktop.host.portal.Registry.Register`.
async fn register(connection: &zbus::Connection) -> std::result::Result<(), zbus::Error> {
    let options: HashMap<&str, Value<'_>> = HashMap::new();
    connection
        .call_method(
            Some("org.freedesktop.portal.Desktop"),
            PORTAL_PATH,
            Some(REGISTRY_INTERFACE),
            "Register",
            &(APP_ID, options),
        )
        .await?;
    tracing::info!("app id registrado no portal: {APP_ID}");
    Ok(())
}

/// Conteúdo do `.desktop` (oculto do menu; existe para o app id do portal).
fn desktop_entry(exec: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=GhostWhisper\n\
         Comment=Meeting copilot\n\
         Exec={}\n\
         Icon=ghostwhisper\n\
         Terminal=false\n\
         NoDisplay=true\n\
         Categories=Utility;\n\
         StartupWMClass=ghostwhisper\n",
        quote_exec(exec)
    )
}

/// Escapa o caminho para o campo `Exec` do `.desktop`.
fn quote_exec(path: &str) -> String {
    if !path.is_empty()
        && path
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "/._-+".contains(character))
    {
        return path.to_string();
    }
    let escaped = path.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// Caminho do `.desktop` para um `XDG_DATA_HOME`/`HOME` de teste.
#[cfg(test)]
fn desktop_path(data_home: &Path) -> PathBuf {
    data_home
        .join("applications")
        .join(format!("{APP_ID}.desktop"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(quote_exec("/usr/bin/ghostwhisper"), "/usr/bin/ghostwhisper");
        assert_eq!(
            quote_exec("/tmp/a b/ghostwhisper"),
            "\"/tmp/a b/ghostwhisper\""
        );
        assert_eq!(quote_exec("/tmp/a\"b"), "\"/tmp/a\\\"b\"");
    }

    #[test]
    fn app_id_is_reverse_dns() {
        assert_eq!(APP_ID, "com.ghostwhisper.app");
        assert!(!APP_ID.ends_with(".desktop"));
    }

    #[test]
    fn desktop_entry_is_hidden_and_matches_the_app_id() {
        let content = desktop_entry("/usr/bin/ghostwhisper");
        assert!(content.contains("Exec=/usr/bin/ghostwhisper"));
        assert!(content.contains("NoDisplay=true"));
        assert!(content.contains("StartupWMClass=ghostwhisper"));
    }

    #[test]
    fn desktop_name_matches_the_app_id() {
        let path = desktop_path(Path::new("/tmp/xdg"));
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("com.ghostwhisper.app.desktop")
        );
    }
}
