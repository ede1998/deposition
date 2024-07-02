include <BOSL/constants.scad>
use <BOSL/masks.scad>
use <BOSL/shapes.scad>

$fn = 20;

container_inner = [65, 25, 12.3];
container_thickness = 3;

connector_gap_height = 3.5;
connector_gap_width = 13.7;

button_height = 11.9;
button_length = 11.9;
button_base_width = 3.7;
button_prongs_distance = 12.5;
button_prongs_length = 2.8;
button_clicker_bump = 3.3;
button_clicker_clearance = 0.3;

button_face = 8;

module brace(lengths, height, thicknesses) {
    assert(len(lengths) == 2, "Requires 2 length values, one for each leg");
    assert(len(thicknesses) == 2, "Requires 2 thickness values, one for each leg");
    linear_extrude(height = height)
        polygon(points = [[0,0], [0, thicknesses.y], [lengths.x, thicknesses.y], [lengths.x, thicknesses.y + lengths.y], [lengths.x + thicknesses.x, thicknesses.y + lengths.y], [lengths.x + thicknesses.x,0]]);
}


module button_brace() {
    brace_thickness = 3;
    button_center_x = button_length / 2 + brace_thickness;
    cover_brace_thickness = [2,2];
    cover_brace_lengths = [3,2.4];
    cover_length = button_length +2;
    cover_brace_start_y = - (button_clicker_bump - brace_thickness);
    
    translate([-button_center_x, cover_brace_lengths.y  - brace_thickness + (button_clicker_bump - button_clicker_clearance), 0]) {
        translate([button_center_x - 0.5 * button_prongs_distance, button_base_width + brace_thickness,0]) cube([button_prongs_distance, button_prongs_length, button_height]);
        extra_support = [button_prongs_distance * 0.7, 4, button_height * 0.8];
        translate([button_center_x - 0.5 * extra_support.x, button_base_width + brace_thickness + button_prongs_length, 0]) cube(extra_support);
        
        translate([2*brace_thickness,0,0]) mirror([1,0,0]) brace([brace_thickness, brace_thickness], button_height, [brace_thickness, brace_thickness - 0.2]);
        translate([button_length,0,0]) brace([brace_thickness, brace_thickness], button_height, [brace_thickness, brace_thickness - 0.2]);
    }
    
    translate([-button_center_x, cover_brace_lengths.y + cover_brace_thickness.y - cover_brace_start_y, 0]) {
        distance_from_center = 0.5 * cover_length - cover_brace_lengths[0];
        translate([button_center_x + distance_from_center, cover_brace_start_y,0]) mirror([0,1,0]) brace(cover_brace_lengths, button_height, cover_brace_thickness);
        translate([button_center_x - distance_from_center, cover_brace_start_y,0]) rotate([0,0,180]) brace(cover_brace_lengths, button_height, cover_brace_thickness);
    }
}

module screw_fixture(diameter, thickness) {
    side = diameter * 1.5;
    translate([-2*side, -side, 0]) difference() {
        cube([side, side, thickness]);
        translate([side/2,side/2, -thickness*0.05]) cylinder(h = thickness * 1.1, d = diameter);
    }
    translate([-side,0,0]) rotate([90,0,0]) right_triangle([side, thickness, side]);
    translate([-2*side,0,0]) rotate([90,0,-90]) right_triangle([side, thickness, side]);
}

module main() {
    translate([0,0, 0.5 * container_inner.z]) difference() {
        cuboid([container_inner.x + container_thickness * 2, container_inner.y + container_thickness * 2, container_inner.z + container_thickness], fillet=2, edges = EDGES_ALL - EDGES_TOP);
        cube(container_inner, center = true);
        translate([0,0, 0.8*container_inner.z]) cube(container_inner, center = true); // Remove roof
        for (i = [-1.5:1.5]) {
            translate([i * (button_length + 4), -0.5*container_inner.y, (button_length + 5)/2-0.5*container_inner.z]) cube([button_face,container_thickness*2.5, button_face+5], center = true);
        }
        translate([0, container_inner.y / 2 - 0.5 * container_thickness, container_inner.z / 2 + container_thickness - connector_gap_height + 0.25])
            backcube([connector_gap_width, container_thickness * 2, connector_gap_height]);
    }
    
    button_start_y = -container_inner[1] /2;
    for (i = [-1.5:1.5]) {
        translate([i * (button_length + 4), button_start_y, 0]) button_brace();
    }
    
    diameter = 5;
    thickness = 4;
    screw_fixture_position_xy = [container_inner.x / 2 + container_thickness, 4.5 * 0.5 * diameter];
    screw_fixture_position_z = container_inner.z - thickness + container_thickness / 2;
    translate(concat(screw_fixture_position_xy, screw_fixture_position_z))
        rotate([0,0,90])
            screw_fixture(diameter = diameter, thickness = thickness);
    translate(concat(-1 *screw_fixture_position_xy, screw_fixture_position_z))
        rotate([0,0,270])
            screw_fixture(diameter = diameter, thickness = thickness);
}

intersection() {
    main();
//    translate([5.5,-7,-2]) cube([button_length+8,24,button_height+container_thickness + 30], center = true);
//    #translate([3.5,width/2+3,-5.5]) rotate([0,0,180]) cube([spool_fixture_distance + 6, wall_thickness * 5, 5]); // spool
//    #translate([23.5,width/2-15,8.5]) cube([7, wall_thickness * 10, 5]); // optocoupler
}


