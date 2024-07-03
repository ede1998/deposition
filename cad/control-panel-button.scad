include <BOSL/constants.scad>
use <BOSL/transforms.scad>
use <BOSL/shapes.scad>

$fn = 20;

touch_area = [7.5, 7.5, 5];
engraving_depth = 1.6;
retainer = [13,13,1.5];
engraving =
    //"1";
    //"2";
    //"▲";
    "▼";

upcube(retainer);
up(retainer.z) {
    difference() {
        upcube(touch_area);
        up(touch_area.z) mirror([0,0,1]) linear_extrude(engraving_depth) text(engraving, size = 6.5, valign = "center", halign = "center");
    }
}