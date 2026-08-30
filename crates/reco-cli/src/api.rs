use owo_colors::OwoColorize;
use reco_core::apis::{
    generate_client, write_client_kit, ApiEndpoint, ApiRegistry, ClientKind,
};
use reco_core::{clients_dir, GgufQuant, Recommendation, Scores};

use crate::run;
use crate::server::{self, HubSlot};

#[allow(clippy::too_many_arguments)]
pub fn create(
    rec: &Recommendation,
    name: Option<&str>,
    provider: &str,
    lan: bool,
    port: Option<u16>,
    write: bool,
    start: bool,
    demo: bool,
) -> Result<(), String> {
    let default_name = rec.repo_id.rsplit('/').next().unwrap_or(&rec.repo_id);
    let name = name.unwrap_or(default_name);
    let mut reg = ApiRegistry::load();
    let quant = rec.quant.label();
    let ep = reg.create(
        name,
        &rec.repo_id,
        &rec.filename,
        &quant,
        provider,
        lan,
        port,
    )?;
    let kit = if write {
        Some(write_client_kit(&ep)?)
    } else {
        None
    };
    print_created(&ep, kit.as_ref().map(|p| p.display().to_string()));
    if start {
        start_named(Some(&ep.slug), None, None, demo)?;
    }
    Ok(())
}

pub fn list(json: bool) -> Result<(), String> {
    let reg = ApiRegistry::load();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&reg).map_err(|err| err.to_string())?
        );
        return Ok(());
    }
    if reg.endpoints.is_empty() {
        println!("{}", "No hay APIs todavía.".bold());
        println!(
            "  {}",
            "reco api create Qwen2.5-7B --name mi-app".dimmed()
        );
        return Ok(());
    }
    println!(
        "{}  {}  ·  tu máquina es el servidor",
        "APIs".bold(),
        reco_core::apis_path().display().to_string().dimmed()
    );
    println!();
    for ep in &reg.endpoints {
        println!(
            "  {}  {}  ·  {}",
            ep.slug.cyan().bold(),
            ep.repo_id,
            ep.quant
        );
        println!(
            "     {}  ·  {}  ·  reco api start {}",
            ep.base_url().dimmed(),
            ep.masked_key().dimmed(),
            ep.slug
        );
    }
    println!();
    println!(
        "  {}",
        "reco api start          hub con todas  ·  reco api code <nombre>".dimmed()
    );
    Ok(())
}

pub fn show(slug: &str, json: bool) -> Result<(), String> {
    let reg = ApiRegistry::load();
    let ep = reg
        .get(slug)
        .ok_or_else(|| format!("no existe la API '{slug}'"))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(ep).map_err(|err| err.to_string())?
        );
        return Ok(());
    }
    print_created(ep, Some(clients_dir().join(&ep.slug).display().to_string()));
    Ok(())
}

pub fn code(slug: &str, client: Option<&str>, write: bool) -> Result<(), String> {
    let reg = ApiRegistry::load();
    let ep = reg
        .get(slug)
        .ok_or_else(|| format!("no existe la API '{slug}'"))?;
    let base = ep.base_url();
    if write {
        let dir = write_client_kit(ep)?;
        println!("clientes en {}", dir.display());
    }
    match client {
        Some(name) => {
            let kind = ClientKind::parse(name)?;
            println!("{}", generate_client(ep, kind, &base));
        }
        None => {
            for kind in [
                ClientKind::Curl,
                ClientKind::Python,
                ClientKind::Continue,
                ClientKind::Env,
            ] {
                println!("{}  {}", "──".dimmed(), kind.filename());
                println!("{}", generate_client(ep, kind, &base));
            }
            println!(
                "{}",
                "Más: reco api code {slug} --client js|cursor|openwebui|langchain|openapi".dimmed()
            );
        }
    }
    Ok(())
}

pub fn rotate(slug: &str) -> Result<(), String> {
    let mut reg = ApiRegistry::load();
    {
        let ep = reg
            .get_mut(slug)
            .ok_or_else(|| format!("no existe la API '{slug}'"))?;
        ep.rotate_key();
    }
    reg.save()?;
    let ep = reg.get(slug).unwrap();
    let _ = write_client_kit(ep);
    println!("nueva clave para {}: {}", ep.slug, ep.api_key);
    Ok(())
}

pub fn rm(slug: &str) -> Result<(), String> {
    let mut reg = ApiRegistry::load();
    let ep = reg
        .remove(slug)
        .ok_or_else(|| format!("no existe la API '{slug}'"))?;
    reg.save()?;
    let dir = clients_dir().join(&ep.slug);
    let _ = std::fs::remove_dir_all(&dir);
    println!("borrada {}", ep.slug);
    Ok(())
}

pub fn start_named(
    slug: Option<&str>,
    host: Option<&str>,
    port: Option<u16>,
    demo: bool,
) -> Result<(), String> {
    let reg = ApiRegistry::load();
    let selected: Vec<ApiEndpoint> = match slug {
        Some(name) => vec![reg
            .get(name)
            .cloned()
            .ok_or_else(|| format!("no existe la API '{name}'"))?],
        None => {
            if reg.endpoints.is_empty() {
                return Err(
                    "no hay APIs. Crea una: reco api create <modelo> --name mi-app".into(),
                );
            }
            reg.endpoints.clone()
        }
    };
    let mut slots = Vec::new();
    for ep in selected {
        slots.push(slot_from_api(&ep, demo)?);
    }
    let bind_host = host
        .map(str::to_string)
        .unwrap_or_else(|| {
            if slots.iter().any(|s| s.api.lan) {
                "0.0.0.0".into()
            } else {
                "127.0.0.1".into()
            }
        });
    let bind_port = port.unwrap_or(slots[0].api.port);
    server::run_hub(&bind_host, bind_port, slots)
}

pub fn slot_from_api(ep: &ApiEndpoint, demo: bool) -> Result<HubSlot, String> {
    let rec = rec_from_api(ep);
    let picked = run::resolve_engine(&rec, demo, &ep.provider)?;
    Ok(HubSlot {
        api: ep.clone(),
        engine: picked.engine,
        label: picked.label,
    })
}

pub fn rec_from_api(ep: &ApiEndpoint) -> Recommendation {
    Recommendation {
        repo_id: ep.repo_id.clone(),
        filename: ep.filename.clone(),
        quant: GgufQuant::parse(&ep.filename)
            .or_else(|| GgufQuant::parse(&ep.quant))
            .unwrap_or(GgufQuant::Q4Km),
        size_bytes: 0,
        size_estimated: true,
        params: None,
        downloads: 0,
        scores: Scores {
            compatibility: 0.0,
            speed: 0.0,
            quality: 0.0,
            popularity: 0.0,
        },
        total: 0.0,
        why: String::new(),
    }
}

fn print_created(ep: &ApiEndpoint, kit: Option<String>) {
    println!("{}", "API lista — tu máquina es el servidor".bold());
    println!("  nombre   {}", ep.name.cyan());
    println!("  slug     {}", ep.slug);
    println!("  modelo   {}  ·  {}", ep.repo_id, ep.quant);
    println!("  URL      {}/v1", ep.base_url());
    println!("  API key  {}", ep.api_key);
    if let Some(kit) = kit {
        println!("  kit      {kit}");
    }
    println!();
    println!("{}", generate_client(ep, ClientKind::Curl, &ep.base_url()));
    println!("  reco api start {:<12}  enciende el servidor", ep.slug.cyan());
    println!(
        "  reco api code {} --client python   otra app en Python",
        ep.slug
    );
    println!(
        "  reco api code {} --client continue  pega en Continue",
        ep.slug
    );
}
