use hapi_rs::ffi::{GeoInfo, ObjectInfo, PartInfo};
use hapi_rs::geometry::{AttributeOwner, CurveInfo, CurveOrders, CurveType, Geometry};
use hapi_rs::session::{new_in_process, SessionOptions};
use hapi_rs::{NodeFlags, NodeType, PartType, Result};

fn main() -> Result<()> {
    let otl = "otls/nurbs_curve.hda";
    let mut session = new_in_process()?;
    let mut opt = SessionOptions::default();
    opt.threaded = true;
    session.initialize(&opt)?;
    let lib = session.load_asset_file(otl)?;
    let node = lib.try_create_first()?;
    node.cook_blocking(None)?;

    let obj_info = &node.get_objects_info()?[0];

    let children = node.get_children(NodeType::Sop, NodeFlags::Curve, true)?;
    for node_h in children {
        let node = node_h.to_node(&session)?;
        let geo = node.geometry()?.expect("geometry");
        let geo_info = geo.geo_info()?;

        for part_info in geo.partitions()? {
            if part_info.part_type() == PartType::Curve {
                print_curve_info(&geo, &obj_info, &part_info);
            }
        }
    }

    Ok(())
}

fn print_curve_info(geo: &Geometry, obj: &ObjectInfo, part: &PartInfo) -> Result<()> {
    let part_id = part.part_id();
    println!(
        "Object Node: {}, Geometry: {}, Part ID: {}",
        obj.node_id().0,
        geo.info.node_id().0,
        part_id
    );

    let curve = geo.curve_info(part_id)?;

    println!(
        "Curve mesh type = {}",
        match curve.curve_type() {
            CurveType::Linear => "Linear",
            CurveType::Nurbs => "Nurbs",
            CurveType::Bezier => "Bezier",
            _ => "Unknown",
        }
    );

    let counts = geo.curve_counts(part_id, 0, curve.curve_count())?;
    println!("Curve count: {}", curve.curve_count());
    let mut knot_offset = 0;
    for (curve_idx, vertices) in counts.iter().enumerate() {
        println!("Curve {} of {}", curve_idx, counts.len());
        println!("Num of vertices : {}", vertices);
        let mut order = curve.order();
        if order == CurveOrders::Varying as i32 || order == CurveOrders::Invalid as i32 {
            let orders = geo.curve_orders(part_id, curve_idx as i32, 1)?;
            order = orders[0];
        };
        println!("Curve Order: {}", order as u32);

        if *vertices < order {
            println!("Not enough vertices on curve {}", curve_idx);
            knot_offset += *vertices + order;
            continue;
        }
        let attrib = geo
            .get_attribute::<f32>(part_id, AttributeOwner::Point, "P")?
            .unwrap();
        let positions = attrib.read(part_id)?;
        for cv in 0..attrib.info.count() {
            let idx = (cv * attrib.info.tuple_size()) as usize;
            println!("CV {}: {:?}", cv + 1, &positions[idx..idx + 3])
        }

        if curve.has_knots() {
            geo.curve_knots(part_id, knot_offset, *vertices + order)?
                .iter()
                .enumerate()
                .for_each(|(i, v)| println!("knot {}: {}", i + 1, v))
        }

        knot_offset += *vertices + order;
    }
    Ok(())
}
