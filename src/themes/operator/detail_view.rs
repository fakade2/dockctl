use crate::app::{App, PanelId};
use crate::display::{
    container_display_name, format_bytes, format_ports, format_timestamp_age, short_id,
};

/// Build describe lines for the currently selected item based on resource view.
pub(super) fn describe_selected(app: &App) -> (String, Vec<String>) {
    match app.operator_resource.as_str() {
        "containers" => {
            let idx = app.panel_selection(PanelId::Containers);
            if let Some(c) = app.containers.get(idx) {
                let title = format!("Container: {}", container_display_name(&c.names));
                let lines = vec![
                    format!("Name:    {}", container_display_name(&c.names)),
                    format!("Image:   {}", c.image),
                    format!("State:   {}", c.state),
                    format!("Status:  {}", c.status),
                    format!("ID:      {}", short_id(&c.id)),
                    format!("Ports:   {}", format_ports(&c.ports)),
                    format!("Command: {}", c.command),
                ];
                (title, lines)
            } else {
                (
                    "No Selection".to_string(),
                    vec!["No container selected".to_string()],
                )
            }
        }
        "images" => {
            let idx = app.panel_selection(PanelId::Images);
            if let Some(img) = app.images.get(idx) {
                let tag = img
                    .repo_tags
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "<none>:<none>".to_string());
                let title = format!("Image: {}", tag);
                let lines = vec![
                    format!("Tags:    {}", img.repo_tags.join(", ")),
                    format!("ID:      {}", short_id(&img.id)),
                    format!("Size:    {}", format_bytes(img.size)),
                    format!("Created: {}", format_timestamp_age(img.created)),
                ];
                (title, lines)
            } else {
                (
                    "No Selection".to_string(),
                    vec!["No image selected".to_string()],
                )
            }
        }
        "volumes" => {
            let idx = app.panel_selection(PanelId::Volumes);
            if let Some(vol) = app.volumes.get(idx) {
                let title = format!("Volume: {}", vol.name);
                let lines = vec![
                    format!("Name:       {}", vol.name),
                    format!("Driver:     {}", vol.driver),
                    format!("Mountpoint: {}", vol.mountpoint),
                    format!("Scope:      {}", vol.scope),
                    format!("Created:    {}", vol.created_at),
                ];
                (title, lines)
            } else {
                (
                    "No Selection".to_string(),
                    vec!["No volume selected".to_string()],
                )
            }
        }
        "networks" => {
            let idx = app.panel_selection(PanelId::Networks);
            if let Some(net) = app.networks.get(idx) {
                let title = format!("Network: {}", net.name);
                let mut lines = vec![
                    format!("Name:   {}", net.name),
                    format!("ID:     {}", short_id(&net.id)),
                    format!("Driver: {}", net.driver),
                    format!("Scope:  {}", net.scope),
                ];
                if !net.containers.is_empty() {
                    lines.push(String::new());
                    lines.push("Connected Containers:".to_string());
                    for (cid, cname) in &net.containers {
                        lines.push(format!("  {} {}", short_id(cid), cname));
                    }
                }
                (title, lines)
            } else {
                (
                    "No Selection".to_string(),
                    vec!["No network selected".to_string()],
                )
            }
        }
        _ => ("Unknown".to_string(), vec!["Unknown resource".to_string()]),
    }
}
