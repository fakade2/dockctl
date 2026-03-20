use crate::app::{Alert, AlertLevel, App};
use crate::runtime::{
    ContainerDetail, ContainerSummary, ImageSummary, NetworkSummary, VolumeSummary,
};
use crate::ui::{
    container_display_name, format_bytes, format_ports, format_timestamp_age, short_id,
};

// ─── Popup content helpers (return Vec<String> for use with App::open_popup) ─

/// Detail lines for an alert (shown in the explain popup).
pub fn alert_detail_lines(alert: &Alert) -> Vec<String> {
    let level = match alert.level {
        AlertLevel::Warning => "Warning",
        AlertLevel::Critical => "CRITICAL",
    };
    vec![
        format!("Level:   {}", level),
        format!("Message: {}", alert.message),
    ]
}

/// Detail lines for the Resources panel (counters for containers, images, etc.).
pub fn resource_detail_lines(app: &App, selected: usize) -> Vec<String> {
    match selected {
        0 => {
            let total = app.containers.len();
            let running = app
                .containers
                .iter()
                .filter(|c| c.state == "running")
                .count();
            let stopped = total - running;
            vec![
                "Containers".to_string(),
                format!("  Total:   {}", total),
                format!("  Running: {}", running),
                format!("  Stopped: {}", stopped),
            ]
        }
        1 => {
            let total = app.images.len();
            let dangling = app.dangling_images_count;
            vec![
                "Images".to_string(),
                format!("  Total:    {}", total),
                format!("  Dangling: {}", dangling),
            ]
        }
        2 => {
            vec![
                "Volumes".to_string(),
                format!("  Total: {}", app.volumes.len()),
            ]
        }
        3 => {
            vec![
                "Networks".to_string(),
                format!("  Total: {}", app.networks.len()),
            ]
        }
        _ => vec!["No detail available".to_string()],
    }
}

/// Detail lines for the Status panel (system-level information).
pub fn status_detail_lines(app: &App, selected: usize) -> Vec<String> {
    if let Some(info) = &app.system_info {
        match selected {
            0 => vec![
                "CPU".to_string(),
                format!("  Cores: {}", info.cpus),
                format!("  Arch:  {}", info.arch),
            ],
            1 => vec![
                "Memory".to_string(),
                format!("  Total: {}", format_bytes(info.memory_total)),
            ],
            2 => vec![
                "System".to_string(),
                format!("  Runtime: {} {}", info.runtime_type, info.server_version),
                format!("  OS:      {}", info.os),
                format!("  Kernel:  {}", info.kernel_version),
                format!("  Storage: {}", info.storage_driver),
            ],
            _ => vec!["No detail available".to_string()],
        }
    } else {
        vec!["System info not available".to_string()]
    }
}

/// Full detail lines for a container (from ContainerDetail).
pub fn container_detail_lines(detail: &ContainerDetail) -> Vec<String> {
    let mut lines = vec![
        format!("Name:       {}", detail.name),
        format!("ID:         {}", short_id(&detail.id)),
        format!("Image:      {}", detail.image),
        format!("State:      {}", detail.state.status),
        format!("PID:        {}", detail.state.pid),
        format!("Created:    {}", detail.created),
        format!("Restarts:   {}", detail.restart_count),
        format!("Driver:     {}", detail.driver),
        format!("Platform:   {}", detail.platform),
        format!("Log path:   {}", detail.log_path),
        String::new(),
    ];

    let full_cmd = if detail.args.is_empty() {
        detail.path.clone()
    } else {
        format!("{} {}", detail.path, detail.args.join(" "))
    };
    lines.push(format!("Command:    {}", full_cmd));
    lines.push(String::new());

    lines.push(format!(
        "Running:    {}",
        if detail.state.running { "yes" } else { "no" }
    ));
    lines.push(format!(
        "Paused:     {}",
        if detail.state.paused { "yes" } else { "no" }
    ));
    lines.push(format!("ExitCode:   {}", detail.state.exit_code));
    lines.push(format!("StartedAt:  {}", detail.state.started_at));
    if !detail.state.finished_at.is_empty() && detail.state.finished_at != "0001-01-01T00:00:00Z" {
        lines.push(format!("FinishedAt: {}", detail.state.finished_at));
    }
    lines.push(String::new());

    if !detail.env.is_empty() {
        lines.push("[Environment]".to_string());
        for env in &detail.env {
            lines.push(format!("  {}", env));
        }
        lines.push(String::new());
    }

    if !detail.labels.is_empty() {
        lines.push("[Labels]".to_string());
        for (k, v) in &detail.labels {
            lines.push(format!("  {}={}", k, v));
        }
    }

    lines
}

/// Brief summary lines for a container (from ContainerSummary).
pub fn container_summary_lines(c: &ContainerSummary) -> Vec<String> {
    vec![
        format!("Name:   {}", container_display_name(&c.names)),
        format!("Image:  {}", c.image),
        format!("State:  {}", c.state),
        format!("Status: {}", c.status),
        format!("Ports:  {}", format_ports(&c.ports)),
    ]
}

/// Detail lines for an image.
pub fn image_detail_lines(img: &ImageSummary) -> Vec<String> {
    let repo_tags = if img.repo_tags.is_empty() {
        "<none>:<none>".to_string()
    } else {
        img.repo_tags.join(", ")
    };

    let mut lines = vec![
        format!("Tags:    {}", repo_tags),
        format!("ID:      {}", short_id(&img.id)),
        format!("Size:    {}", format_bytes(img.size)),
        format!("Created: {}", format_timestamp_age(img.created)),
        String::new(),
    ];

    if !img.repo_digests.is_empty() {
        lines.push("[Digests]".to_string());
        for digest in &img.repo_digests {
            lines.push(format!("  {}", digest));
        }
        lines.push(String::new());
    }

    if !img.labels.is_empty() {
        lines.push("[Labels]".to_string());
        for (k, v) in &img.labels {
            lines.push(format!("  {}={}", k, v));
        }
    }

    lines
}

/// Detail lines for a volume.
pub fn volume_detail_lines(vol: &VolumeSummary) -> Vec<String> {
    vec![
        format!("Name:       {}", vol.name),
        format!("Driver:     {}", vol.driver),
        format!("Mountpoint: {}", vol.mountpoint),
        format!("Scope:      {}", vol.scope),
        format!("Created:    {}", vol.created_at),
    ]
}

/// Detail lines for a network.
pub fn network_detail_lines(net: &NetworkSummary) -> Vec<String> {
    let mut lines = vec![
        format!("Name:   {}", net.name),
        format!("ID:     {}", short_id(&net.id)),
        format!("Driver: {}", net.driver),
        format!("Scope:  {}", net.scope),
        String::new(),
    ];

    if !net.containers.is_empty() {
        lines.push("[Connected Containers]".to_string());
        for (cid, cname) in &net.containers {
            lines.push(format!("  {} {}", short_id(cid), cname));
        }
    }

    lines
}
