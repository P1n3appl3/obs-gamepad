# nix eval --json --file gram-layout.nix | yj -jt > gram-layout.toml
let
  blue = "#5BCEFA";
  pink = "#F5A9B8";
  white = "#FFFFFF";
  alpha = "42";
  btn = id: pos: color: {
    id = id;
    pos = map (n: n*2) pos;
    fill_active = color; outline_active = color;
    fill = color + alpha; outline = color;
  };
in {
  outline_weight = 3.4;
  button_shape.radius = 21.8;
  buttons = [
    (btn 0  [ 201.3   102.03 ] white)   # Start
    (btn 5  [ 69.13   96.58  ] blue)    # L
    (btn 11 [ 96.85   84.93  ] pink)    # Left
    (btn 9  [ 125.26  87.18  ] white)   # Down
    (btn 10 [ 149.33  103.61 ] pink)    # Right
    (btn 6  [ 247.55  78.68  ] blue)    # R
    (btn 1  [ 271.77  62.53  ] pink)    # Y
    (btn 18 [ 300.1   60.66  ] white)   # Light-shield
    (btn 19 [ 328.69  71.8   ] pink)    # Mid-shield
    (btn 3  [ 252.41  106.18 ] blue)    # B
    (btn 2  [ 276.44  89.76  ] pink)    # X
    (btn 7  [ 304.87  87.76  ] white)   # Z
    (btn 8  [ 333.5   98.98  ] pink)    # Up
    (btn 12 [ 145.16  167.24 ] blue)    # Mod-x
    (btn 13 [ 164.77  186.71 ] pink)    # Mod-y
    (btn 14 [ 222.94  158.79 ] white)   # C-left
    (btn 16 [ 242.56  139.19 ] blue)    # C-up
    (btn 17 [ 230.98  186.06 ] blue)    # C-down
    (btn 4  [ 250.57  166.58 ] pink)    # A
    (btn 15 [ 270.21  147.46 ] white)   # C-right
    (btn 20 [ 129.84  61.17  ] blue)    # D-pad toggle
  ];
}
