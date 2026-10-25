//! Arraste/redimensionamento nativo das janelas sem moldura.
//!
//! O `start_dragging` do Tauri chama `gtk_window_begin_move_drag` com timestamp
//! zero. Em Mutter/X11 o primeiro gesto de uma janela inativa é ignorado e o
//! pedido seguinte pode cair no fim de um grab anterior (ISS-063). Aqui usamos
//! a mesma API do GTK, mas com o **timestamp atual do servidor X11**, que é o
//! que o WM espera.

#[cfg(target_os = "linux")]
mod imp {
    use gdkx11::X11Window;
    use gtk::prelude::*;

    use crate::error::{GhostWhisperError, Result};

    /// Direções aceitas pelo frontend (`WindowResizeDirection`).
    #[cfg(test)]
    pub const RESIZE_DIRECTIONS: [&str; 8] = [
        "East",
        "North",
        "NorthEast",
        "NorthWest",
        "South",
        "SouthEast",
        "SouthWest",
        "West",
    ];

    pub fn edge_for(direction: &str) -> Option<gtk::gdk::WindowEdge> {
        use gtk::gdk::WindowEdge;
        match direction {
            "North" => Some(WindowEdge::North),
            "NorthEast" => Some(WindowEdge::NorthEast),
            "NorthWest" => Some(WindowEdge::NorthWest),
            "West" => Some(WindowEdge::West),
            "East" => Some(WindowEdge::East),
            "SouthWest" => Some(WindowEdge::SouthWest),
            "South" => Some(WindowEdge::South),
            "SouthEast" => Some(WindowEdge::SouthEast),
            _ => None,
        }
    }

    /// Devolve a janela GTK, a posição global do ponteiro e o tempo do servidor.
    ///
    /// Em Wayland não há XID/tempo do X11; o timestamp fica 0 e o próprio GTK
    /// conduz o gesto pelo compositor.
    fn pointer_context(
        window: &tauri::WebviewWindow,
    ) -> Result<(gtk::ApplicationWindow, i32, i32, u32)> {
        let gtk_window = window
            .gtk_window()
            .map_err(|error| GhostWhisperError::Other(format!("janela GTK: {error}")))?;
        let gdk_window = gtk_window
            .window()
            .ok_or_else(|| GhostWhisperError::Other("janela GTK sem GdkWindow".into()))?;
        let timestamp = gdk_window
            .clone()
            .downcast::<X11Window>()
            .map(|x11| gdkx11::functions::x11_get_server_time(&x11))
            .unwrap_or(0);

        let seat = gdk_window
            .display()
            .default_seat()
            .ok_or_else(|| GhostWhisperError::Other("sem seat do display".into()))?;
        let pointer = seat
            .pointer()
            .ok_or_else(|| GhostWhisperError::Other("sem ponteiro do seat".into()))?;
        let (_screen, root_x, root_y) = pointer.position();

        Ok((gtk_window, root_x, root_y, timestamp))
    }

    pub fn start_drag(window: tauri::WebviewWindow) -> Result<()> {
        let (gtk_window, root_x, root_y, timestamp) = pointer_context(&window)?;
        gtk_window.begin_move_drag(1, root_x, root_y, timestamp);
        Ok(())
    }

    pub fn start_resize(window: tauri::WebviewWindow, direction: &str) -> Result<()> {
        let edge = edge_for(direction).ok_or_else(|| {
            GhostWhisperError::Config(format!("direção de resize desconhecida: {direction}"))
        })?;
        let (gtk_window, root_x, root_y, timestamp) = pointer_context(&window)?;
        gtk_window.begin_resize_drag(edge, 1, root_x, root_y, timestamp);
        Ok(())
    }
}

/// Inicia o arraste da janela chamadora com timestamp X11 válido.
#[tauri::command]
pub fn window_start_drag(window: tauri::WebviewWindow) -> crate::error::Result<()> {
    #[cfg(target_os = "linux")]
    {
        match imp::start_drag(window.clone()) {
            Ok(()) => Ok(()),
            // Sem GTK (raro): usa o caminho do Tauri.
            Err(_) => window
                .start_dragging()
                .map_err(|error| crate::error::GhostWhisperError::Other(error.to_string())),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        window
            .start_dragging()
            .map_err(|error| crate::error::GhostWhisperError::Other(error.to_string()))
    }
}

/// Inicia o redimensionamento da janela chamadora com timestamp X11 válido.
#[tauri::command]
pub fn window_start_resize(
    window: tauri::WebviewWindow,
    direction: String,
) -> crate::error::Result<()> {
    #[cfg(target_os = "linux")]
    {
        match imp::start_resize(window.clone(), &direction) {
            Ok(()) => Ok(()),
            Err(error) => Err(error),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let edge = match direction.as_str() {
            "North" => tauri_runtime::ResizeDirection::North,
            "NorthEast" => tauri_runtime::ResizeDirection::NorthEast,
            "NorthWest" => tauri_runtime::ResizeDirection::NorthWest,
            "West" => tauri_runtime::ResizeDirection::West,
            "East" => tauri_runtime::ResizeDirection::East,
            "SouthWest" => tauri_runtime::ResizeDirection::SouthWest,
            "South" => tauri_runtime::ResizeDirection::South,
            "SouthEast" => tauri_runtime::ResizeDirection::SouthEast,
            _ => {
                return Err(crate::error::GhostWhisperError::Config(format!(
                    "direção de resize desconhecida: {direction}"
                )))
            }
        };
        window
            .start_resize_dragging(edge)
            .map_err(|error| crate::error::GhostWhisperError::Other(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_os = "linux")]
    fn resize_directions_all_map_to_an_edge() {
        for direction in super::imp::RESIZE_DIRECTIONS {
            assert!(
                super::imp::edge_for(direction).is_some(),
                "direção sem edge: {direction}"
            );
        }
    }
}
