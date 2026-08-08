use toml::{Table, Value};

use super::{
    LevelPackageError, PACKAGE_SCHEMA, PackageResult, WORLD_CHECKPOINTS_PATH, WORLD_DEBUG_PATH,
    WORLD_GROUPS_PATH, WORLD_INDEX_PATH, WORLD_LAYERS_PATH, WORLD_PORTALS_PATH, WORLD_SOURCE_PATH,
    WORLD_TRIGGERS_PATH, error::invalid_data, quote,
};

#[derive(Clone, Debug)]
pub struct LevelManifest {
    pub id: String,
    pub title: String,
    pub author: String,
    pub entry: String,
    pub chunk_size_units: i32,
    pub coord_scale: i32,
}
pub(super) fn format_manifest(manifest: &LevelManifest) -> String {
    format!(
        "[package]\nschema = {PACKAGE_SCHEMA}\nid = {}\ntitle = {}\nauthor = {}\ncreated_with = \"aperture-kill\"\nengine_min = \"0.1.0\"\nentry = {}\n\n[world]\nmode = \"chunked\"\nencoding = \"typed_bitpacked_soa\"\nsource = \"{WORLD_SOURCE_PATH}\"\ndebug_dump = \"{WORLD_DEBUG_PATH}\"\nindex = \"{WORLD_INDEX_PATH}\"\nlayers = \"{WORLD_LAYERS_PATH}\"\ngroups = \"{WORLD_GROUPS_PATH}\"\ntriggers = \"{WORLD_TRIGGERS_PATH}\"\ncheckpoints = \"{WORLD_CHECKPOINTS_PATH}\"\nportals = \"{WORLD_PORTALS_PATH}\"\nchunk_size_units = {}\ncoord_scale = {}\ncoord_storage = \"chunk_local_i16\"\nchunk_origin_storage = \"world_i32\"\n\n[bits]\nobject_kind_bits = 8\nportalable_bits = 1\nsurface_type_bits = 2\nview_layer_bits = 3\ngroup_id_bits = 10\neditor_layer_bits = 10\n\n[visibility]\nmodel = \"screen_culling\"\nrender_guard_units = 256\nparticle_guard_units = 512\nmax_target_aspect = \"48:9\"\nmax_target_visible_chunks = 256\nocclusion_culling = \"optional\"\n\n[simulation]\nmodel = \"interest_volumes\"\nplayer_margin_units = 768\nprojectile_margin_units = 1024\nportal_physics_margin_units = 512\ncheckpoint_pin_radius_units = 1024\nsleep_offscreen_actors = true\n\n[compression]\nchunk_default = \"none\"\npreferred = \"none\"\n",
        quote(&manifest.id),
        quote(&manifest.title),
        quote(&manifest.author),
        quote(&manifest.entry),
        manifest.chunk_size_units,
        manifest.coord_scale
    )
}

pub(super) fn parse_manifest(source: &str) -> PackageResult<LevelManifest> {
    let values = source
        .parse::<Table>()
        .map_err(|err| invalid_data(format!("invalid manifest.toml: {err}")))?;
    let values = Value::Table(values);
    let schema = required_manifest_i32(&values, &["package", "schema"])?;
    let schema = u8::try_from(schema)
        .map_err(|_| invalid_data(format!("unsupported manifest schema {schema}")))?;
    if schema != PACKAGE_SCHEMA {
        return Err(LevelPackageError::UnsupportedSchema { schema });
    }
    let encoding = required_manifest_string(&values, &["world", "encoding"])?;
    if encoding != "typed_bitpacked_soa" {
        return Err(LevelPackageError::UnsupportedEncoding { encoding });
    }

    Ok(LevelManifest {
        id: required_manifest_string(&values, &["package", "id"])?,
        title: required_manifest_string(&values, &["package", "title"])?,
        author: required_manifest_string(&values, &["package", "author"])?,
        entry: required_manifest_string(&values, &["package", "entry"])?,
        chunk_size_units: required_manifest_i32(&values, &["world", "chunk_size_units"])?,
        coord_scale: required_manifest_i32(&values, &["world", "coord_scale"])?,
    })
}

fn required_manifest_string(values: &Value, path: &[&str]) -> PackageResult<String> {
    manifest_value(values, path)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| invalid_data(format!("manifest {} must be a string", path.join("."))))
}

fn required_manifest_i32(values: &Value, path: &[&str]) -> PackageResult<i32> {
    let value = manifest_value(values, path)?
        .as_integer()
        .ok_or_else(|| invalid_data(format!("manifest {} must be an integer", path.join("."))))?;

    i32::try_from(value)
        .map_err(|_| invalid_data(format!("manifest {} does not fit i32", path.join("."))))
}

fn manifest_value<'a>(values: &'a Value, path: &[&str]) -> PackageResult<&'a Value> {
    let mut current = values;

    for segment in path {
        current = current
            .get(*segment)
            .ok_or_else(|| invalid_data(format!("manifest missing {}", path.join("."))))?;
    }

    Ok(current)
}

pub(super) fn stable_level_id(name: &str) -> String {
    let slug = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '.'
            }
        })
        .collect::<String>();

    format!("author.level.{}", slug.trim_matches('.').replace("..", "."))
}
