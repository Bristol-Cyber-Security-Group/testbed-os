use lazy_static::lazy_static;
use tera::Tera;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("*.xml") {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        let _ = tera.add_raw_template("libvirt_network", include_str!("templates/libvirt_network.xml"));
        let _ = tera.add_raw_template("libvirt_domain", include_str!("templates/libvirt_domain.xml"));
        tera
    };
}

pub fn render_libvirt_network_xml(
    tera_context: tera::Context,
) -> anyhow::Result<String> {
    let render = TEMPLATES.render("libvirt_network", &tera_context)?;
    Ok(render)
}

pub fn render_libvirt_domain_xml(
    tera_context: tera::Context,
) -> anyhow::Result<String> {
    let render = TEMPLATES.render("libvirt_domain", &tera_context)?;
    Ok(render)
}
