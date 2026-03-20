use crate::app::{Alert, AlertLevel, App};
use crate::display::{
    container_display_name, format_bytes, format_ports, format_timestamp_age, short_id,
};
use crate::runtime::{
    ContainerDetail, ContainerSummary, ImageSummary, NetworkSummary, VolumeSummary,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Alert, AlertLevel, App};
    use crate::runtime::*;
    use std::collections::HashMap;

    fn make_app() -> App {
        let mut app = App::new(false);
        app.containers = vec![ContainerSummary {
            id: "abc123".into(),
            name: "nginx".into(),
            names: vec!["nginx".into()],
            image: "nginx:latest".into(),
            image_id: "sha256:abc".into(),
            command: "nginx -g daemon off;".into(),
            state: "running".into(),
            status: "Up 5 days".into(),
            created: 1700000000,
            ports: vec![],
            labels: HashMap::new(),
        }];
        app.images = vec![ImageSummary {
            id: "sha256:def456".into(),
            repo_tags: vec!["nginx:latest".into()],
            repo_digests: vec![],
            size: 187000000,
            created: 1699999000,
            labels: HashMap::new(),
        }];
        app.volumes = vec![VolumeSummary {
            name: "pgdata".into(),
            driver: "local".into(),
            mountpoint: "/var/lib/docker/volumes/pgdata".into(),
            created_at: "2026-01-01".into(),
            labels: HashMap::new(),
            scope: "local".into(),
        }];
        app.networks = vec![NetworkSummary {
            id: "net123".into(),
            name: "bridge".into(),
            driver: "bridge".into(),
            scope: "local".into(),
            internal: false,
            ipv6_enabled: false,
            containers: HashMap::new(),
            labels: HashMap::new(),
        }];
        app
    }

    // alert_detail_lines
    #[test]
    fn alert_warning() {
        let alert = Alert {
            level: AlertLevel::Warning,
            message: "test warning".into(),
        };
        let lines = alert_detail_lines(&alert);
        assert!(lines.iter().any(|l| l.contains("Warning")));
        assert!(lines.iter().any(|l| l.contains("test warning")));
    }
    #[test]
    fn alert_critical() {
        let alert = Alert {
            level: AlertLevel::Critical,
            message: "critical!".into(),
        };
        let lines = alert_detail_lines(&alert);
        assert!(lines.iter().any(|l| l.contains("CRITICAL")));
    }

    // resource_detail_lines
    #[test]
    fn resource_containers() {
        let app = make_app();
        let lines = resource_detail_lines(&app, 0);
        assert!(lines.iter().any(|l| l.contains("Containers")));
    }
    #[test]
    fn resource_images() {
        let app = make_app();
        let lines = resource_detail_lines(&app, 1);
        assert!(lines.iter().any(|l| l.contains("Images")));
    }
    #[test]
    fn resource_volumes() {
        let app = make_app();
        let lines = resource_detail_lines(&app, 2);
        assert!(lines.iter().any(|l| l.contains("Volumes")));
    }
    #[test]
    fn resource_networks() {
        let app = make_app();
        let lines = resource_detail_lines(&app, 3);
        assert!(lines.iter().any(|l| l.contains("Networks")));
    }
    #[test]
    fn resource_out_of_range() {
        let app = make_app();
        let lines = resource_detail_lines(&app, 99);
        assert!(lines.iter().any(|l| l.contains("No detail")));
    }

    // status_detail_lines
    #[test]
    fn status_no_info() {
        let app = make_app();
        let lines = status_detail_lines(&app, 0);
        assert!(lines.iter().any(|l| l.contains("not available")));
    }
    #[test]
    fn status_with_info_cpu() {
        let mut app = make_app();
        app.system_info = Some(SystemInfo {
            runtime_type: RuntimeType::Docker,
            server_version: "24.0".into(),
            api_version: "1.43".into(),
            os: "linux".into(),
            arch: "amd64".into(),
            kernel_version: "6.1.0".into(),
            memory_total: 16 * 1024 * 1024 * 1024,
            cpus: 8,
            containers_total: 5,
            containers_running: 3,
            containers_paused: 0,
            containers_stopped: 2,
            images: 10,
            storage_driver: "overlay2".into(),
        });
        let lines = status_detail_lines(&app, 0);
        assert!(lines.iter().any(|l| l.contains("CPU")));
        assert!(lines.iter().any(|l| l.contains("8")));
    }

    // container_detail_lines
    #[test]
    fn container_detail() {
        let detail = ContainerDetail {
            id: "abc".into(),
            name: "nginx".into(),
            image: "nginx:latest".into(),
            created: "2026-01-01".into(),
            state: ContainerState {
                status: "running".into(),
                running: true,
                paused: false,
                restarting: false,
                oom_killed: false,
                dead: false,
                pid: 1234,
                exit_code: 0,
                started_at: "2026-01-01".into(),
                finished_at: "".into(),
            },
            path: "/usr/sbin/nginx".into(),
            args: vec!["-g".into(), "daemon off;".into()],
            restart_count: 0,
            driver: "overlay2".into(),
            platform: "linux".into(),
            log_path: "/var/log".into(),
            labels: HashMap::new(),
            env: vec!["PORT=80".into()],
        };
        let lines = container_detail_lines(&detail);
        assert!(lines.iter().any(|l| l.contains("nginx")));
        assert!(lines.iter().any(|l| l.contains("running")));
        assert!(lines.iter().any(|l| l.contains("PORT=80")));
    }
    #[test]
    fn container_detail_no_args() {
        let detail = ContainerDetail {
            id: "abc".into(),
            name: "test".into(),
            image: "alpine:latest".into(),
            created: "2026-01-01".into(),
            state: ContainerState {
                status: "exited".into(),
                running: false,
                paused: false,
                restarting: false,
                oom_killed: false,
                dead: false,
                pid: 0,
                exit_code: 1,
                started_at: "2026-01-01".into(),
                finished_at: "2026-01-02".into(),
            },
            path: "/bin/sh".into(),
            args: vec![],
            restart_count: 3,
            driver: "overlay2".into(),
            platform: "linux".into(),
            log_path: "/var/log".into(),
            labels: HashMap::from([("app".into(), "test".into())]),
            env: vec![],
        };
        let lines = container_detail_lines(&detail);
        assert!(lines.iter().any(|l| l.contains("/bin/sh")));
        assert!(lines.iter().any(|l| l.contains("FinishedAt")));
        assert!(lines.iter().any(|l| l.contains("[Labels]")));
        assert!(lines.iter().any(|l| l.contains("app=test")));
    }

    // container_summary_lines
    #[test]
    fn container_summary() {
        let app = make_app();
        let lines = container_summary_lines(&app.containers[0]);
        assert!(lines.iter().any(|l| l.contains("nginx")));
        assert!(lines.iter().any(|l| l.contains("running")));
    }

    // image_detail_lines
    #[test]
    fn image_detail() {
        let app = make_app();
        let lines = image_detail_lines(&app.images[0]);
        assert!(lines.iter().any(|l| l.contains("nginx:latest")));
        assert!(lines.iter().any(|l| l.contains("MB")));
    }
    #[test]
    fn image_detail_no_tags() {
        let img = ImageSummary {
            id: "sha256:abc".into(),
            repo_tags: vec![],
            repo_digests: vec!["sha256:digest123".into()],
            size: 1024,
            created: 1699999000,
            labels: HashMap::new(),
        };
        let lines = image_detail_lines(&img);
        assert!(lines.iter().any(|l| l.contains("<none>:<none>")));
        assert!(lines.iter().any(|l| l.contains("[Digests]")));
    }

    // volume_detail_lines
    #[test]
    fn volume_detail() {
        let app = make_app();
        let lines = volume_detail_lines(&app.volumes[0]);
        assert!(lines.iter().any(|l| l.contains("pgdata")));
        assert!(lines.iter().any(|l| l.contains("local")));
    }

    // network_detail_lines
    #[test]
    fn network_detail() {
        let app = make_app();
        let lines = network_detail_lines(&app.networks[0]);
        assert!(lines.iter().any(|l| l.contains("bridge")));
    }
    #[test]
    fn network_with_containers() {
        let mut app = make_app();
        app.networks[0]
            .containers
            .insert("c1".into(), "nginx".into());
        let lines = network_detail_lines(&app.networks[0]);
        assert!(lines.iter().any(|l| l.contains("nginx")));
        assert!(lines.iter().any(|l| l.contains("[Connected Containers]")));
    }
}
