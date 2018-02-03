local hidpp = Proto("hidpp", "Logitech HID++")

local lengths = {
    [0x10] = "Short",
    [0x11] = "Long",
    [0x12] = "Very Long",
}

local pf_msg_len = ProtoField.uint8("hidpp.msg_len", "Message Length", base.HEX, lengths)
local pf_device_id = ProtoField.new("Device ID", "hidpp.device_id", ftypes.UINT8, nul, base.HEX)
local pf_feature = ProtoField.new("Feature Index", "hidpp.feature", ftypes.UINT8, nul, base.HEX)
local pf_fnid = ProtoField.new("Function ID", "hidpp.fnid", ftypes.UINT8, nul, base.HEX)
local pf_swid = ProtoField.new("Software ID", "hidpp.swid", ftypes.UINT8, nul, base.HEX)
local pf_data = ProtoField.new("Data", "hidpp.data", ftypes.BYTES)

hidpp.fields = {
    pf_msg_len,
    pf_device_id,
    pf_feature,
    pf_fnid,
    pf_swid,
    pf_data,
}

hidpp.dissector = function(tvbuf, pktinfo, root)
    pktinfo.cols.protocol:set("HID++")

    local tree = root:add(hidpp, tvbuf:range(0, tvbuf:len()))

    tree:add(pf_msg_len, tvbuf:range(0, 1))
    tree:add(pf_device_id, tvbuf:range(1, 1))
    tree:add(pf_feature, tvbuf:range(2, 1):le_uint())
    tree:add(pf_fnid, tvbuf:range(3, 1):bitfield(0, 4))
    tree:add(pf_swid, tvbuf:range(3, 1):bitfield(4, 4))
    tree:add(pf_data, tvbuf:range(4, tvbuf:len() - 4))
end

local function heuristic_dissector(tvbuf, pktinfo, root)
    -- Checks the packet length and makes sure it's one of short, long, very_long
    if not (tvbuf:len() == 20) then
        return false
    end

    -- Checks against the first byte (length descriptor)
    if not (tvbuf:range(0, 1):le_uint() == 0x11) then
        return false
    end

    -- Okay, it's our packet probably. Call the dissector.
    hidpp.dissector(tvbuf, pktinfo, root)
    return true
end

local function heuristic_dissector_control(tvbuf, pktinfo, root)
    tvbuf = tvbuf:range(7, tvbuf:len() - 7):tvb()
    heuristic_dissector(tvbuf, pktinfo, root)
end

hidpp:register_heuristic("usb.interrupt", heuristic_dissector)
hidpp:register_heuristic("usb.control", heuristic_dissector_control)
