pub static MP1V_MACROBLOCK_ADDRESS_INCREMENT: [(i16, i16); 80] = [
    (  1 << 1,    0), (       0,    1),  //   0: x
    (  2 << 1,    0), (  3 << 1,    0),  //   1: 0x
    (  4 << 1,    0), (  5 << 1,    0),  //   2: 00x
    (       0,    3), (       0,    2),  //   3: 01x
    (  6 << 1,    0), (  7 << 1,    0),  //   4: 000x
    (       0,    5), (       0,    4),  //   5: 001x
    (  8 << 1,    0), (  9 << 1,    0),  //   6: 0000x
    (       0,    7), (       0,    6),  //   7: 0001x
    ( 10 << 1,    0), ( 11 << 1,    0),  //   8: 0000 0x
    ( 12 << 1,    0), ( 13 << 1,    0),  //   9: 0000 1x
    ( 14 << 1,    0), ( 15 << 1,    0),  //  10: 0000 00x
    ( 16 << 1,    0), ( 17 << 1,    0),  //  11: 0000 01x
    ( 18 << 1,    0), ( 19 << 1,    0),  //  12: 0000 10x
    (       0,    9), (       0,    8),  //  13: 0000 11x
    (      -1,    0), ( 20 << 1,    0),  //  14: 0000 000x
    (      -1,    0), ( 21 << 1,    0),  //  15: 0000 001x
    ( 22 << 1,    0), ( 23 << 1,    0),  //  16: 0000 010x
    (       0,   15), (       0,   14),  //  17: 0000 011x
    (       0,   13), (       0,   12),  //  18: 0000 100x
    (       0,   11), (       0,   10),  //  19: 0000 101x
    ( 24 << 1,    0), ( 25 << 1,    0),  //  20: 0000 0001x
    ( 26 << 1,    0), ( 27 << 1,    0),  //  21: 0000 0011x
    ( 28 << 1,    0), ( 29 << 1,    0),  //  22: 0000 0100x
    ( 30 << 1,    0), ( 31 << 1,    0),  //  23: 0000 0101x
    ( 32 << 1,    0), (      -1,    0),  //  24: 0000 0001 0x
    (      -1,    0), ( 33 << 1,    0),  //  25: 0000 0001 1x
    ( 34 << 1,    0), ( 35 << 1,    0),  //  26: 0000 0011 0x
    ( 36 << 1,    0), ( 37 << 1,    0),  //  27: 0000 0011 1x
    ( 38 << 1,    0), ( 39 << 1,    0),  //  28: 0000 0100 0x
    (       0,   21), (       0,   20),  //  29: 0000 0100 1x
    (       0,   19), (       0,   18),  //  30: 0000 0101 0x
    (       0,   17), (       0,   16),  //  31: 0000 0101 1x
    (       0,   35), (      -1,    0),  //  32: 0000 0001 00x
    (      -1,    0), (       0,   34),  //  33: 0000 0001 11x
    (       0,   33), (       0,   32),  //  34: 0000 0011 00x
    (       0,   31), (       0,   30),  //  35: 0000 0011 01x
    (       0,   29), (       0,   28),  //  36: 0000 0011 10x
    (       0,   27), (       0,   26),  //  37: 0000 0011 11x
    (       0,   25), (       0,   24),  //  38: 0000 0100 00x
    (       0,   23), (       0,   22),  //  39: 0000 0100 01x
];

pub static MP1V_MACROBLOCK_TYPE_INTRA: [(i16, i16); 4]  = [
    (  1 << 1,    0), (       0,  0x01),  //   0: x
    (      -1,    0), (       0,  0x11),  //   1: 0x
];

pub static MP1V_MACROBLOCK_TYPE_PREDICTIVE: [(i16, i16); 14]  = [
    (  1 << 1,    0), (       0, 0x0a),  //   0: x
    (  2 << 1,    0), (       0, 0x02),  //   1: 0x
    (  3 << 1,    0), (       0, 0x08),  //   2: 00x
    (  4 << 1,    0), (  5 << 1,    0),  //   3: 000x
    (  6 << 1,    0), (       0, 0x12),  //   4: 0000x
    (       0, 0x1a), (       0, 0x01),  //   5: 0001x
    (      -1,    0), (       0, 0x11),  //   6: 0000 0x
];

pub static MP1V_CODE_BLOCK_PATTERN: [(i16, i16); 14] = [
    {  1 << 1,    0}, {  2 << 1,    0},  //   0: x
    {  3 << 1,    0}, {  4 << 1,    0},  //   1: 0x
    {  5 << 1,    0}, {  6 << 1,    0},  //   2: 1x
    {  7 << 1,    0}, {  8 << 1,    0},  //   3: 00x
    {  9 << 1,    0}, { 10 << 1,    0},  //   4: 01x
    { 11 << 1,    0}, { 12 << 1,    0},  //   5: 10x
    { 13 << 1,    0}, {       0,   60},  //   6: 11x
    { 14 << 1,    0}, { 15 << 1,    0},  //   7: 000x
    { 16 << 1,    0}, { 17 << 1,    0},  //   8: 001x
    { 18 << 1,    0}, { 19 << 1,    0},  //   9: 010x
    { 20 << 1,    0}, { 21 << 1,    0},  //  10: 011x
    { 22 << 1,    0}, { 23 << 1,    0},  //  11: 100x
    {       0,   32}, {       0,   16},  //  12: 101x
    {       0,    8}, {       0,    4},  //  13: 110x
    { 24 << 1,    0}, { 25 << 1,    0},  //  14: 0000x
    { 26 << 1,    0}, { 27 << 1,    0},  //  15: 0001x
    { 28 << 1,    0}, { 29 << 1,    0},  //  16: 0010x
    { 30 << 1,    0}, { 31 << 1,    0},  //  17: 0011x
    {       0,   62}, {       0,    2},  //  18: 0100x
    {       0,   61}, {       0,    1},  //  19: 0101x
    {       0,   56}, {       0,   52},  //  20: 0110x
    {       0,   44}, {       0,   28},  //  21: 0111x
    {       0,   40}, {       0,   20},  //  22: 1000x
    {       0,   48}, {       0,   12},  //  23: 1001x
    { 32 << 1,    0}, { 33 << 1,    0},  //  24: 0000 0x
    { 34 << 1,    0}, { 35 << 1,    0},  //  25: 0000 1x
    { 36 << 1,    0}, { 37 << 1,    0},  //  26: 0001 0x
    { 38 << 1,    0}, { 39 << 1,    0},  //  27: 0001 1x
    { 40 << 1,    0}, { 41 << 1,    0},  //  28: 0010 0x
    { 42 << 1,    0}, { 43 << 1,    0},  //  29: 0010 1x

];
