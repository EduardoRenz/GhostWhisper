//! Monitores no X11 via **RandR 1.5** (`GetMonitors`), com fallback para a tela
//! virtual (Xinerama clássico).
//!
//! Usado em dois lugares:
//! - gravação: escolher um monitor específico (além da tela inteira);
//! - print de tela: capturar o monitor **sob o cursor** (BR-055).

use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

/// Um monitor físico/lógico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub primary: bool,
}

impl Monitor {
    /// `true` quando o ponto cai dentro do monitor.
    pub fn contains(&self, x: i16, y: i16) -> bool {
        let right = i32::from(self.x) + i32::from(self.width);
        let bottom = i32::from(self.y) + i32::from(self.height);
        i32::from(x) >= i32::from(self.x)
            && i32::from(x) < right
            && i32::from(y) >= i32::from(self.y)
            && i32::from(y) < bottom
    }
}

/// Lista os monitores; sem RandR 1.5, devolve a tela virtual como um só.
pub fn monitors() -> Vec<Monitor> {
    let Some((connection, screen_num)) = RustConnection::connect(None).ok() else {
        return Vec::new();
    };
    monitors_from(&connection, screen_num)
}

fn monitors_from(connection: &RustConnection, screen_num: usize) -> Vec<Monitor> {
    let root = connection.setup().roots[screen_num].root;

    let version = connection
        .randr_query_version(1, 5)
        .ok()
        .and_then(|cookie| cookie.reply().ok());
    let supports_1_5 = version
        .map(|reply| reply.major_version > 1 || (reply.major_version == 1 && reply.minor_version >= 5))
        .unwrap_or(false);

    if supports_1_5 {
        if let Some(reply) = connection
            .randr_get_monitors(root, true)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
        {
            let mut out: Vec<Monitor> = reply
                .monitors
                .iter()
                .enumerate()
                .map(|(index, monitor)| Monitor {
                    name: atom_name(connection, monitor.name)
                        .unwrap_or_else(|| format!("Monitor {}", index + 1)),
                    x: monitor.x,
                    y: monitor.y,
                    width: monitor.width,
                    height: monitor.height,
                    primary: monitor.primary,
                })
                .collect();
            out.sort_by_key(|monitor| (monitor.x, monitor.y));
            if !out.is_empty() {
                return out;
            }
        }
    }

    let screen = &connection.setup().roots[screen_num];
    vec![Monitor {
        name: "Tela inteira".to_string(),
        x: 0,
        y: 0,
        width: screen.width_in_pixels,
        height: screen.height_in_pixels,
        primary: true,
    }]
}

fn atom_name(connection: &RustConnection, atom: u32) -> Option<String> {
    let reply = connection.get_atom_name(atom).ok()?.reply().ok()?;
    let name = String::from_utf8_lossy(&reply.name).trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// Monitor que contém o ponto; senão o primário; senão o primeiro.
pub fn monitor_at(monitors: &[Monitor], x: i16, y: i16) -> Option<Monitor> {
    monitors
        .iter()
        .find(|monitor| monitor.contains(x, y))
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
        .or_else(|| monitors.first())
        .cloned()
}

/// Monitor sob o cursor (fallback: primário). Abre uma conexão própria.
pub fn monitor_under_pointer() -> Option<Monitor> {
    let (connection, screen_num) = RustConnection::connect(None).ok()?;
    let root = connection.setup().roots[screen_num].root;
    let pointer = connection.query_pointer(root).ok()?.reply().ok()?;
    let monitors = monitors_from(&connection, screen_num);
    monitor_at(&monitors, pointer.root_x, pointer.root_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: i16, width: u16, primary: bool) -> Monitor {
        Monitor {
            name: name.to_string(),
            x,
            y: 0,
            width,
            height: 1080,
            primary,
        }
    }

    #[test]
    fn contains_respects_the_origin() {
        let second = monitor("HDMI-1", 1920, 1920, false);
        assert!(!second.contains(1919, 100));
        assert!(second.contains(1920, 0));
        assert!(second.contains(3839, 1079));
        assert!(!second.contains(3840, 100));
    }

    #[test]
    fn monitor_at_picks_the_monitor_under_the_point() {
        let monitors = vec![monitor("DP-1", 0, 1920, true), monitor("HDMI-1", 1920, 1920, false)];
        assert_eq!(monitor_at(&monitors, 100, 100).expect("1").name, "DP-1");
        assert_eq!(
            monitor_at(&monitors, 2000, 100).expect("2").name,
            "HDMI-1"
        );
    }

    #[test]
    fn monitor_at_falls_back_to_primary_then_first() {
        let monitors = vec![monitor("A", 0, 1920, false), monitor("B", 1920, 1920, true)];
        // Fora de todos: usa o primário.
        assert_eq!(monitor_at(&monitors, 9000, 9000).expect("primário").name, "B");

        let no_primary = vec![monitor("A", 0, 1920, false)];
        assert_eq!(monitor_at(&no_primary, 9000, 9000).expect("primeiro").name, "A");
        assert!(monitor_at(&[], 0, 0).is_none());
    }
}
