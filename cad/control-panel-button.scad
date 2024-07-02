include <BOSL/constants.scad>
use <BOSL/transforms.scad>
use <BOSL/shapes.scad>

$fn = 20;

touch_area = [7.5, 7.5, 5];
retainer = [13,11,1.5];

upcube(retainer);
up(retainer.z) #upcube(touch_area);