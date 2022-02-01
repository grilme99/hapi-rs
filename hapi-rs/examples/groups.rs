// Port of groups.cpp
// Example demonstrates creating nodes, geometry, creating and querying point groups
use hapi_rs::geometry::{
    AttributeInfo, AttributeOwner, GroupType, PartInfo, PartType, StorageType,
};
use hapi_rs::node::HoudiniNode;
use hapi_rs::parameter::{Parameter, ParmBaseTrait};
use hapi_rs::session::{quick_session, Session, SessionOptions};
use hapi_rs::Result;

fn create_cube(session: &Session) -> Result<HoudiniNode> {
    let cube_node = session.create_input_node("Cube")?;
    cube_node.cook(None)?;

    let part_info = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_face_count(6)
        .with_vertex_count(24)
        .with_point_count(8);

    let geometry = cube_node.geometry()?.unwrap();
    geometry.set_part_info(&part_info)?;

    let attr_info = AttributeInfo::default()
        .with_count(8)
        .with_tuple_size(3)
        .with_storage(StorageType::Float)
        .with_owner(AttributeOwner::Point);

    let p_attr = geometry.add_numeric_attribute("P", 0, attr_info)?;

    #[rustfmt::skip]
        let positions = [
        0.0f32, 0.0, 0.0,
        0.0, 0.0, 1.0,
        0.0, 1.0, 0.0,
        0.0, 1.0, 1.0,
        1.0, 0.0, 0.0,
        1.0, 0.0, 1.0,
        1.0, 1.0, 0.0,
        1.0, 1.0, 1.0
    ];

    p_attr.set(0, &positions)?;

    #[rustfmt::skip]
        let vertices = [
        0, 2, 6, 4,
        2, 3, 7, 6,
        2, 0, 1, 3,
        1, 5, 7, 3,
        5, 4, 6, 7,
        0, 4, 5, 1
    ];
    geometry.set_vertex_list(0, &vertices)?;
    geometry.set_face_counts(0, &[4, 4, 4, 4, 4, 4])?;

    let num_elem = part_info.element_count_by_group(GroupType::Point);
    let membership: Vec<_> = (0..num_elem)
        .map(|v| if (v % 2) > 0 { 1 } else { 0 })
        .collect();
    geometry.add_group(0, GroupType::Point, "pointGroup", Some(&membership))?;
    geometry.commit()?;
    Ok(cube_node)
}

fn main() -> Result<()> {
    let mut session = quick_session()?;
    session.initialize(&SessionOptions::default())?;
    let cube = create_cube(&session)?;
    let xform = session.create_node("Sop/xform", Some("PointGroupManipulator"), None)?;
    xform.connect_input(0, &cube, 0)?;

    if let Parameter::String(p) = xform.parameter("group").expect("group parm") {
        p.set_value(&["pointGroup".to_owned()])?;
    }

    if let Parameter::Float(p) = xform.parameter("t").expect("t parm") {
        p.set_value(&[0.0, 1.0, 0.0])?
    }
    xform.cook(None)?;

    let geo = xform.geometry()?.unwrap();
    let num_groups = geo.group_count_by_type(GroupType::Point, None)?;
    println!("Number of point groups on xform: {}", num_groups);

    let part = geo.part_info(0)?;
    let num_points = part.element_count_by_group(GroupType::Point);
    println!("{} points in pointGroup", num_points);

    let membership = geo.get_group_membership(Some(&part), GroupType::Point, "pointGroup")?;

    for (pt, member) in membership.into_iter().enumerate() {
        if member > 0 {
            println!("Point {} is in pointGroup", pt)
        }
    }
    let hip = std::env::temp_dir().join("groups.hip");
    session.save_hip(&hip.to_string_lossy(), true)?;
    println!("Saving {}", hip.to_string_lossy());
    Ok(())
}
