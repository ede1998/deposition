include <BOSL/constants.scad>
use <BOSL/masks.scad>
use <BOSL/shapes.scad>

$fn = 20;
wall_thickness = 2;
length = 110;
width = 45;
height = 40;

esp_mount_hole_diameter = 2.3;
esp_mount_hole_distance_long = 50.7;
esp_mount_hole_distance_short = 23.4;
esp_mount_hole_distance_front = 1.5;

spool_fixture_distance = 38.7;
spool_fixture_diameter = 2.2;
yarn_hole_radius = 1;
yarn_hole_distance_from_bottom = 10;
yarn_hole_distance_from_closer_fixture = 8.4;

usb_hole_width = 9.3;
usb_hole_height = 4;

module spool() {
    rotate([-90,0,-90]) {
        import("minimal-case-body_lid.stl");
        import("minimal-case-body_body.stl");
        translate([0,0,-19.8]) cylinder(h = 19.8, d = 22.3);
    }
}
module esp() {
    cube([54.6, 28.3, 13.0], center = true);
}

module optocouplers() {
    cube([31, 8, 24], center = true);
}

%translate([27,0,-10]) union() {
    esp();
    translate([-61,0,7]) spool();
    translate([0, width/2 - 4, 15]) optocouplers();
}

module spool_fixtures(height) {
    rotate([90, 0, 0]) union() {
        cylinder(h = height, d = spool_fixture_diameter);
        translate([-spool_fixture_distance, 0, 0]) cylinder(h = height, d = spool_fixture_diameter);
    }
}

module yarn_hole() {
    hull_bottom = -(height + wall_thickness) / 2;
    pos_short = width/2 - yarn_hole_distance_from_bottom;
    pos_long = -yarn_hole_distance_from_closer_fixture+spool_fixture_diameter/2;
    translate([pos_long, pos_short, hull_bottom - wall_thickness * 0.5])
        cylinder(h = wall_thickness * 1.5, r = yarn_hole_radius);
}

module usb_hole() {
    translate([length/2,0,-(height-usb_hole_height)/2])
        cube([wall_thickness*1.5, usb_hole_width, usb_hole_height], center = true);
}

module esp_mount_fixtures(fixture_height) {
    translate([wall_thickness + esp_mount_hole_distance_front - esp_mount_hole_diameter/2 + 0.2, -esp_mount_hole_distance_short / 2, -height/2])
        union() {
            cylinder(h = fixture_height, d = esp_mount_hole_diameter);
            translate([esp_mount_hole_distance_long,0,0]) cylinder(h = fixture_height, d = esp_mount_hole_diameter);
            translate([0,esp_mount_hole_distance_short,0]) cylinder(h = fixture_height, d = esp_mount_hole_diameter);
            translate([esp_mount_hole_distance_long,esp_mount_hole_distance_short,0]) cylinder(h = fixture_height, d = esp_mount_hole_diameter);
        }
}

module optocouplers_fixture(fixture_height) {
    rotate([90,0,0]) cylinder(h = fixture_height, d = 3.3);
}

module cables_hole(diameter) {
    rotate([0,90,0]) translate([0,0,-wall_thickness]) cylinder(h = wall_thickness * 1.5, d = diameter);
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
    // basic hull
    let (
        container_inner = [length, width, height],
        container_outer = [ for (i = container_inner) i + wall_thickness ]
    ) {
        difference() {
            fillet(fillet=1.5, size=container_outer, edges=EDGES_Z_ALL, $fn=24) {
                cube(container_outer, center = true); // outer wall
            }
            cube(container_inner, center = true); // inner empty space
            translate([0, 0, height / 2]) cube([length, width, height], center = true); // remove top
            yarn_hole();
            usb_hole();
            translate([-length/2,-0.7 * width/2,0]) cables_hole(diameter = 5);
        }
    }
    
    translate([1, width/2, -3]) spool_fixtures(height = 6);
    esp_mount_fixtures(fixture_height = 8);
    translate([27, width/2, 11]) optocouplers_fixture(fixture_height = 5);
    
    screw_fixture_position_xy = [length / 2 - wall_thickness, - 0.5 * (width + wall_thickness)];
    screw_fixture_position_z = height / 2 - 2;
    translate(concat(screw_fixture_position_xy, screw_fixture_position_z)) screw_fixture(diameter = 3, thickness = 3);
    translate(concat(-1 *screw_fixture_position_xy, screw_fixture_position_z))
        rotate([0,0,180])
            screw_fixture(diameter = 3, thickness = 3);
}

intersection() {
    main();
//    #translate([1,-width/2+9,-height/2-wall_thickness]) cube([esp_mount_hole_distance_long + 6,esp_mount_hole_distance_short + 4,wall_thickness * 5]); // esp
//    #translate([3.5,width/2+3,-5.5]) rotate([0,0,180]) cube([spool_fixture_distance + 6, wall_thickness * 5, 5]); // spool
//    #translate([23.5,width/2-15,8.5]) cube([7, wall_thickness * 10, 5]); // optocoupler
}


