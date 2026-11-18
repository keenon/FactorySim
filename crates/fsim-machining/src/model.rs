//! Procedural 3D models of machining cells, generated from catalog spec dimensions,
//! the installed configuration bundle, and the cell's estimated run state.
//!
//! Local frame: origin at the footprint min corner, x along width, y along depth with the
//! operator-facing front at y = 0, z up. Every part stays inside the declared
//! `width x depth x height` envelope (checked in tests for all catalog entries and add-on sets).
//!
//! Named nodes (viewer state-binding contract): `base`, `enclosure`, `door`, `window`,
//! `spindle`, `table`, `pendant`, `screen`, `status_light`, plus add-on nodes `probe`,
//! `chip_conveyor`, `pallet_changer`, `bar_feeder`, and kind-specific `trunnion`, `chuck`, `turret`.

use crate::MachiningCellConfig;
use fsim_core::{CellCatalogEntry, CellModel, ModelAxis, ModelMaterial, ModelPart, ModelShape};
use fsim_protocols::CellRunState;

const ENCLOSURE_WHITE: &str = "#eef0ee";
const BASE_GRAY: &str = "#8a949c";
const PENDANT_DARK: &str = "#4a535c";
const SCREEN_BLUE: &str = "#8cc7e8";
const METAL: &str = "#b8c0c6";
const GLASS: &str = "#bfe3ee";
const LIGHT_OFF: &str = "#d4d8da";
const GREEN_ON: &str = "#6cc070";
const AMBER_ON: &str = "#e8b04b";
const RED_ON: &str = "#e06c5f";

/// Topology family of a machining cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineKind {
    Vmc,
    FiveAxis,
    Lathe,
    Compact,
}

impl MachineKind {
    pub fn from_catalog_id(catalog_id: &str) -> Self {
        match catalog_id {
            "cnc-5axis-trunnion" => Self::FiveAxis,
            "cnc-turning-lathe" => Self::Lathe,
            "cnc-compact-mill" => Self::Compact,
            _ => Self::Vmc,
        }
    }

    fn accent(self) -> &'static str {
        match self {
            Self::Vmc => "#5fa8a0",
            Self::FiveAxis => "#7b9acc",
            Self::Lathe => "#d9976a",
            Self::Compact => "#9bbf6a",
        }
    }
}

struct Builder {
    parts: Vec<ModelPart>,
}

impl Builder {
    /// Adds a box from its min and max corners.
    fn bx(&mut self, node: &str, lo: [i32; 3], hi: [i32; 3], material: ModelMaterial, color: &str) {
        let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        if size.iter().any(|&s| s <= 0) {
            return;
        }
        // Center rounded down; size made even so bounds reproduce lo/hi exactly.
        let size = [size[0] & !1, size[1] & !1, size[2] & !1];
        self.parts.push(ModelPart {
            node: node.to_string(),
            shape: ModelShape::Box { size_mm: size },
            center_mm: [lo[0] + size[0] / 2, lo[1] + size[1] / 2, lo[2] + size[2] / 2],
            material,
            color_hex: color.to_string(),
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn cyl(
        &mut self,
        node: &str,
        center: [i32; 3],
        radius: i32,
        length: i32,
        axis: ModelAxis,
        material: ModelMaterial,
        color: &str,
    ) {
        if radius <= 0 || length <= 0 {
            return;
        }
        self.parts.push(ModelPart {
            node: node.to_string(),
            shape: ModelShape::Cylinder {
                radius_mm: radius,
                length_mm: length & !1,
                axis,
            },
            center_mm: center,
            material,
            color_hex: color.to_string(),
        });
    }
}

/// Builds the 3D model of a machining cell from its catalog spec, configuration and run state.
pub fn machining_cell_model(
    catalog: &CellCatalogEntry,
    cfg: &MachiningCellConfig,
    run_state: CellRunState,
) -> CellModel {
    let kind = MachineKind::from_catalog_id(&catalog.catalog_id);
    let w = catalog.width_mm;
    let d = catalog.depth_mm;
    let h = catalog.height_mm;
    let accent = kind.accent();
    let mut b = Builder { parts: Vec::new() };

    // Side bay (pendant + coolant/pallet pool) on the +x side; bar feeder bay on the -x side for lathes.
    let bar_bay = if kind == MachineKind::Lathe && cfg.has_bar_feeder {
        w * 26 / 100
    } else {
        0
    };
    let side_bay = (w * 18 / 100).max(300);
    let ex0 = bar_bay;
    let ex1 = w - side_bay;
    let enc_w = ex1 - ex0;
    let base_h = 260;
    let enc_top = h * 82 / 100;
    let back_y = if cfg.has_chip_conveyor { d * 80 / 100 } else { d };

    // Machine base / skid
    b.bx("base", [ex0, 0, 0], [ex1, back_y, base_h], ModelMaterial::Paint, BASE_GRAY);

    // Sheet-metal enclosure: back and side walls, roof (front is the door + fixed panel)
    let wall = 40;
    b.bx("enclosure", [ex0, back_y - wall, base_h], [ex1, back_y, enc_top], ModelMaterial::Paint, ENCLOSURE_WHITE);
    b.bx("enclosure", [ex0, 0, base_h], [ex0 + wall, back_y, enc_top], ModelMaterial::Paint, ENCLOSURE_WHITE);
    b.bx("enclosure", [ex1 - wall, 0, base_h], [ex1, back_y, enc_top], ModelMaterial::Paint, ENCLOSURE_WHITE);
    b.bx("enclosure", [ex0, 0, enc_top - wall], [ex1, back_y, enc_top], ModelMaterial::Paint, ENCLOSURE_WHITE);
    // Accent stripe across the front header
    let header_lo = enc_top - (enc_top - base_h) * 18 / 100;
    b.bx("enclosure", [ex0, 0, header_lo], [ex1, wall, enc_top - wall], ModelMaterial::Paint, accent);

    // Sliding front door with a window; slides toward +x when not running.
    let door_w = enc_w * 55 / 100;
    let door_lo_z = base_h + 120;
    let door_hi_z = header_lo - 20;
    let door_closed_x0 = ex0 + (enc_w - door_w) / 2;
    let door_open = !matches!(run_state, CellRunState::Running | CellRunState::Installing | CellRunState::Planned);
    let door_x0 = if door_open {
        (ex1 - wall - door_w).max(door_closed_x0)
    } else {
        door_closed_x0
    };
    b.bx("door", [door_x0, wall / 2, door_lo_z], [door_x0 + door_w, wall, door_hi_z], ModelMaterial::Paint, ENCLOSURE_WHITE);
    let win_m = door_w * 15 / 100;
    b.bx(
        "window",
        [door_x0 + win_m, 0, door_lo_z + (door_hi_z - door_lo_z) * 30 / 100],
        [door_x0 + door_w - win_m, wall / 2, door_hi_z - 80],
        ModelMaterial::Glass,
        GLASS,
    );
    // Fixed front panels either side of the door opening (set back behind the sliding door)
    b.bx("enclosure", [ex0, wall, base_h], [door_closed_x0, 2 * wall, header_lo], ModelMaterial::Paint, ENCLOSURE_WHITE);
    b.bx("enclosure", [door_closed_x0 + door_w, wall, base_h], [ex1, 2 * wall, header_lo], ModelMaterial::Paint, ENCLOSURE_WHITE);

    // Work zone internals
    let cx = ex0 + enc_w / 2;
    let cy = back_y * 45 / 100;
    match kind {
        MachineKind::Lathe => {
            // Horizontal spindle: headstock box on -x, chuck cylinder along X, turret on +x.
            let axis_z = base_h + (enc_top - base_h) * 45 / 100;
            b.bx("headstock", [ex0 + wall, cy - 260, base_h], [ex0 + wall + enc_w * 22 / 100, cy + 260, axis_z + 320], ModelMaterial::Paint, accent);
            let chuck_r = 170;
            let chuck_x = ex0 + wall + enc_w * 22 / 100 + 60;
            b.cyl("chuck", [chuck_x, cy, axis_z], chuck_r, 120, ModelAxis::X, ModelMaterial::Metal, METAL);
            b.cyl("spindle", [chuck_x + 170, cy, axis_z], 35, 220, ModelAxis::X, ModelMaterial::Metal, METAL);
            let tur_x = ex0 + enc_w * 62 / 100;
            b.cyl("turret", [tur_x, cy + 80, axis_z], 190, 160, ModelAxis::X, ModelMaterial::Metal, METAL);
            b.bx("table", [ex0 + wall, cy - 300, base_h], [ex1 - wall, cy + 300, base_h + 120], ModelMaterial::Metal, METAL);
        }
        MachineKind::FiveAxis => {
            b.bx("table", [cx - 450, cy - 300, base_h], [cx + 450, cy + 300, base_h + 260], ModelMaterial::Metal, METAL);
            let tr_z = base_h + 520;
            b.cyl("trunnion", [cx, cy, tr_z], 200, enc_w * 50 / 100, ModelAxis::X, ModelMaterial::Paint, accent);
            b.cyl("rotary_table", [cx, cy, tr_z + 230], 230, 60, ModelAxis::Z, ModelMaterial::Metal, METAL);
            b.cyl("spindle", [cx, cy, enc_top - 330], 85, 420, ModelAxis::Z, ModelMaterial::Metal, METAL);
        }
        MachineKind::Vmc | MachineKind::Compact => {
            let tw = enc_w * 30 / 100;
            b.bx("table", [cx - tw, cy - 220, base_h], [cx + tw, cy + 220, base_h + 380], ModelMaterial::Metal, METAL);
            let sp_len = 380;
            b.cyl("spindle", [cx, cy, enc_top - wall - 120 - sp_len / 2], 80, sp_len, ModelAxis::Z, ModelMaterial::Metal, METAL);
        }
    }
    if cfg.has_in_process_probing {
        let pz = (enc_top - 560).max(base_h + 500);
        b.cyl("probe", [cx + 230, cy, pz], 30, 160, ModelAxis::Z, ModelMaterial::Emissive, RED_ON);
    }

    // Spindle head / column rising above the enclosure roof (uses full declared height).
    if kind != MachineKind::Lathe {
        let col_w = enc_w * 32 / 100;
        let col_top = h - 20;
        b.bx("column", [cx - col_w / 2, back_y * 50 / 100, enc_top], [cx + col_w / 2, back_y - wall, col_top], ModelMaterial::Paint, accent);
    }

    // Control pendant on a post in the side bay, angled toward the operator.
    let sb_cx = ex1 + side_bay / 2;
    let pend_w = side_bay * 70 / 100;
    let pend_lo = (h * 42 / 100).max(base_h + 400);
    let pend_hi = (pend_lo + 640).min(enc_top);
    b.cyl("pendant_post", [sb_cx, 220, pend_lo / 2], 40, pend_lo, ModelAxis::Z, ModelMaterial::Metal, METAL);
    b.bx("pendant", [sb_cx - pend_w / 2, 120, pend_lo], [sb_cx + pend_w / 2, 260, pend_hi], ModelMaterial::Paint, PENDANT_DARK);
    b.bx(
        "screen",
        [sb_cx - pend_w * 38 / 100, 100, pend_lo + (pend_hi - pend_lo) * 45 / 100],
        [sb_cx + pend_w * 38 / 100, 120, pend_hi - 60],
        ModelMaterial::Screen,
        SCREEN_BLUE,
    );

    // Side bay rear: pallet pool (5-axis add-on) or coolant tank.
    let bay_back = d * 40 / 100;
    if cfg.has_pallet_changer {
        let r = (side_bay / 2 - 30).min((d - bay_back) / 2 - 30);
        b.cyl("pallet_changer", [sb_cx, bay_back + (d - bay_back) / 2, base_h + 450], r, 900, ModelAxis::Z, ModelMaterial::Metal, METAL);
    } else {
        b.bx("coolant_tank", [ex1 + 40, bay_back, 0], [w - 40, d - 40, 650], ModelMaterial::Paint, BASE_GRAY);
    }

    // Chip conveyor out the back (add-on), with a hopper.
    if cfg.has_chip_conveyor {
        let cv_w = enc_w * 22 / 100;
        b.bx("chip_conveyor", [ex0 + 80, back_y, base_h], [ex0 + 80 + cv_w, d - 20, base_h + 900], ModelMaterial::Metal, METAL);
        b.bx("chip_hopper", [ex0 + 80 + cv_w + 60, back_y + 40, 0], [ex0 + 80 + cv_w * 2 + 60, d - 20, 600], ModelMaterial::Paint, "#7fa7c9");
    }

    // Bar feeder (lathe add-on) feeding along +x into the headstock.
    if bar_bay > 0 {
        let axis_z = base_h + (enc_top - base_h) * 45 / 100;
        b.bx("bar_feeder", [40, cy - 180, 0], [bar_bay - 40, cy + 180, axis_z - 150], ModelMaterial::Paint, accent);
        b.cyl("bar_stock", [bar_bay / 2, cy, axis_z], 25, bar_bay - 120, ModelAxis::X, ModelMaterial::Metal, METAL);
    }

    // Three-segment status light tower on the roof, front +x corner.
    let seg = ((h - enc_top - 10) / 3).clamp(30, 110);
    let (g, a, r) = match run_state {
        CellRunState::Running => (GREEN_ON, LIGHT_OFF, LIGHT_OFF),
        CellRunState::Idle | CellRunState::Setup | CellRunState::Blocked => (LIGHT_OFF, AMBER_ON, LIGHT_OFF),
        CellRunState::Down => (LIGHT_OFF, LIGHT_OFF, RED_ON),
        CellRunState::Installing | CellRunState::Planned => (LIGHT_OFF, LIGHT_OFF, LIGHT_OFF),
    };
    let tx = ex1 - 120;
    let ty = 160;
    for (i, color) in [g, a, r].iter().enumerate() {
        let lit = *color != LIGHT_OFF;
        b.cyl(
            "status_light",
            [tx, ty, enc_top + seg * (i as i32) + seg / 2],
            45,
            seg,
            ModelAxis::Z,
            if lit { ModelMaterial::Emissive } else { ModelMaterial::Paint },
            color,
        );
    }

    CellModel {
        width_mm: w,
        depth_mm: d,
        height_mm: h,
        parts: b.parts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starter_machining_catalog;

    fn all_configs() -> Vec<MachiningCellConfig> {
        let mut out = Vec::new();
        for mask in 0..16u8 {
            out.push(MachiningCellConfig {
                has_bar_feeder: mask & 1 != 0,
                has_in_process_probing: mask & 2 != 0,
                has_pallet_changer: mask & 4 != 0,
                has_chip_conveyor: mask & 8 != 0,
            });
        }
        out
    }

    const STATES: [CellRunState; 7] = [
        CellRunState::Running,
        CellRunState::Setup,
        CellRunState::Idle,
        CellRunState::Blocked,
        CellRunState::Down,
        CellRunState::Planned,
        CellRunState::Installing,
    ];

    #[test]
    fn every_catalog_entry_config_and_state_fits_declared_envelope_with_required_nodes() {
        for cat in starter_machining_catalog() {
            for cfg in all_configs() {
                for st in STATES {
                    let m = machining_cell_model(&cat, &cfg, st);
                    assert!(
                        m.fits_envelope(),
                        "{} {:?} {:?} bounds {:?} exceed {}x{}x{}",
                        cat.catalog_id, cfg, st, m.bounds_mm(), m.width_mm, m.depth_mm, m.height_mm
                    );
                    for node in ["base", "enclosure", "door", "window", "spindle", "table", "pendant", "screen", "status_light"] {
                        assert!(m.part(node).is_some(), "{} missing node {node}", cat.catalog_id);
                    }
                }
            }
        }
    }

    #[test]
    fn addons_add_their_named_parts() {
        let cats = starter_machining_catalog();
        let vmc = cats.iter().find(|c| c.catalog_id == "cnc-3axis-vmc").unwrap();
        let five = cats.iter().find(|c| c.catalog_id == "cnc-5axis-trunnion").unwrap();
        let lathe = cats.iter().find(|c| c.catalog_id == "cnc-turning-lathe").unwrap();
        let plain = MachiningCellConfig::default();
        let full = MachiningCellConfig::from_addons(&[
            "probing".into(),
            "chip-conveyor".into(),
            "pallet-changer".into(),
            "bar-feeder".into(),
        ]);
        let st = CellRunState::Idle;

        let v0 = machining_cell_model(vmc, &plain, st);
        let v1 = machining_cell_model(vmc, &full, st);
        assert!(v0.part("probe").is_none() && v1.part("probe").is_some());
        assert!(v0.part("chip_conveyor").is_none() && v1.part("chip_conveyor").is_some());
        assert!(machining_cell_model(five, &full, st).part("pallet_changer").is_some());
        assert!(machining_cell_model(five, &plain, st).part("trunnion").is_some());
        assert!(machining_cell_model(lathe, &full, st).part("bar_feeder").is_some());
        assert!(machining_cell_model(lathe, &plain, st).part("chuck").is_some());
    }

    #[test]
    fn door_slides_open_when_not_running_and_status_light_reflects_state() {
        let cat = &starter_machining_catalog()[0];
        let cfg = MachiningCellConfig::default();
        let running = machining_cell_model(cat, &cfg, CellRunState::Running);
        let idle = machining_cell_model(cat, &cfg, CellRunState::Idle);
        assert_ne!(running.part("door").unwrap().center_mm, idle.part("door").unwrap().center_mm);

        let lit = |m: &CellModel| {
            m.parts
                .iter()
                .filter(|p| p.node == "status_light" && p.material == ModelMaterial::Emissive)
                .map(|p| p.color_hex.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(lit(&running), vec![GREEN_ON.to_string()]);
        assert_eq!(lit(&idle), vec![AMBER_ON.to_string()]);
        assert!(lit(&machining_cell_model(cat, &cfg, CellRunState::Planned)).is_empty());
    }

    #[test]
    fn model_scales_with_spec_dimensions() {
        let mut cat = starter_machining_catalog()[0].clone();
        let small = machining_cell_model(&cat, &MachiningCellConfig::default(), CellRunState::Idle);
        cat.width_mm *= 2;
        cat.depth_mm *= 2;
        let big = machining_cell_model(&cat, &MachiningCellConfig::default(), CellRunState::Idle);
        let (_, s_hi) = small.bounds_mm().unwrap();
        let (_, b_hi) = big.bounds_mm().unwrap();
        assert!(b_hi[0] > s_hi[0] && b_hi[1] > s_hi[1]);
        assert!(big.fits_envelope());
    }
}
