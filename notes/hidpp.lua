local function dissect_lights(data)
    local light_map = {
        [0] = "logo",
        [1] = "side",
    }
    local light = light_map[data:range(0, 1):uint()]

    local effect
    local effect_type = data:range(1, 1):uint()
    if effect_type == 0 then
        effect = "off"
    end
    if effect_type == 1 then
        local red = data:range(2, 1):uint()
        local green = data:range(3, 1):uint()
        local blue = data:range(4, 1):uint()

        effect = string.format("static { red: 0x%x, green: 0x%x, blue: 0x%x }", red, green, blue)
    end
    if effect_type == 2 then
        local red = data:range(2, 1):uint()
        local green = data:range(3, 1):uint()
        local blue = data:range(4, 1):uint()
        local rate = data:range(5, 2):uint()
        local brightness = data:range(8, 1):uint()

        effect = string.format(
            "breathing { red: 0x%x, green: 0x%x, blue: 0x%x, rate: %u, brightness: %u }",
            red, green, blue, rate, brightness
        )
    end
    if effect_type == 3 then
        local rate = data:range(7, 2):uint()
        local brightness = data:range(9, 1):uint()

        effect = string.format("color_cycle { rate: %u, brightness: %u }", rate, brightness)
    end

    local save_map = {
        [0] = "temporary",
        [2] = "permanent",
    }
    local save = save_map[data:range(12, 1):uint()]

    return light, effect, save
end

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
local pf_desc = ProtoField.new("Description", "hidpp.desc", ftypes.STRING)

hidpp.fields = {
    pf_msg_len,
    pf_device_id,
    pf_feature,
    pf_fnid,
    pf_swid,
    pf_data,
    pf_desc,
}

local function dissector(tvbuf, pktinfo, root)
    pktinfo.cols.protocol:set("HID++")

    local tree = root:add(hidpp, tvbuf:range(0, tvbuf:len()))

    tree:add(pf_msg_len, tvbuf:range(0, 1))
    tree:add(pf_device_id, tvbuf:range(1, 1))
    tree:add(pf_feature, tvbuf:range(2, 1))
    tree:add(pf_fnid, tvbuf:range(3, 1), tvbuf:range(3, 1):bitfield(0, 4))
    tree:add(pf_swid, tvbuf:range(3, 1), tvbuf:range(3, 1):bitfield(4, 4))
    tree:add(pf_data, tvbuf:range(4, tvbuf:len() - 4))

    return tree
end

local function dissector_request(tvbuf, pktinfo, root)
    local tree = dissector(tvbuf, pktinfo, root)

    local feature = tvbuf:range(2, 1):uint()
    local fnid = tvbuf:range(3, 1):bitfield(0, 4)
    local data = tvbuf:range(4, tvbuf:len() - 4)

    -- root
    if feature == 0x00 then
        if fnid == 0x00 then
            local feature_id = data:range(0, 2):uint()

            local desc = string.format("get_feature(feature_id: 0x%x)", feature_id)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x01 then
            local ping_data = data:range(2, 1):uint()

            local desc = string.format("get_protocol_version(ping_data: 0x%x)", ping_data)

            tree:add(pf_desc, desc)
        end
    end

    -- feature set
    if feature == 0x01 then
        if fnid == 0x00 then
            tree:add(pf_desc, "get_feature_count()")
        end
        if fnid == 0x01 then
            local index = data:range(0, 1):uint()

            local desc = string.format("get_feature_id(index: 0x%x)", index)

            tree:add(pf_desc, desc)
        end
    end

    -- device information
    if feature == 0x02 then
        if fnid == 0x00 then
            tree:add(pf_desc, "get_device_info()")
        end
        if fnid == 0x01 then
            local entity_index = data:range(0, 1):uint()

            local desc = string.format("get_fw_info(entity_index: %u)", entity_index)

            tree:add(pf_desc, desc)
        end
    end

    -- device name/type
    if feature == 0x03 then
        if fnid == 0x00 then
            tree:add(pf_desc, "get_device_name_length()")
        end
        if fnid == 0x01 then
            local part_index = data:range(0, 1):uint()

            local desc = string.format("get_device_name(part_index: %u)", part_index)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x02 then
            tree:add(pf_desc, "get_device_type()")
        end
    end

    -- lights
    if feature == 0x04 then
        if fnid == 0x03 then
            local light, effect, save = dissect_lights(data)

            local desc = string.format(
                "set_lights(light: %s, effect: %s, save: %s)",
                light, effect, save
            )

            tree:add(pf_desc, desc)
        end
        if fnid == 0x04 then
            local unknown0 = data:range(0, 1):uint()
            local unknown1 = data:range(1, 1):uint()

            local desc = string.format("get_startup_effect_enabled(??: 0x%x, ??: 0x%x)", unknown0, unknown1)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x05 then
            local unknown0 = data:range(0, 1):uint()
            local unknown1 = data:range(1, 1):uint()
            local enabled = (data:range(2, 1):uint() == 0x01)

            local desc = string.format(
                "enable_startup_effect(??: 0x%x, ??: 0x%x, enabled: %s",
                unknown0, unknown1, enabled
            )

            tree:add(pf_desc, desc)
        end
    end

    -- gkeys
    if feature == 0x05 then
        if fnid == 0x00 then
            tree:add(pf_desc, "get_button_count()")
        end
        if fnid == 0x01 then
            tree:add(pf_desc, "get_buttons_enabled()")
        end
        if fnid == 0x02 then
            local enabled = (data:range(0, 1):uint() == 0x01)

            local desc = string.format("enable_buttons(enabled: %s)", enabled)

            tree:add(pf_desc, desc)
        end
    end

    -- equalizer
    if feature == 0x06 then
        if fnid == 0x00 then
            tree:add(pf_desc, "get_equalizer_info()")
        end
        if fnid == 0x01 then
            local start_index = data:range(0, 1):uint()

            local desc = string.format("get_equalizer_bands(start_index: %u)", start_index)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x02 then
            tree:add(pf_desc, "get_equalizer()")
        end
        if fnid == 0x03 then
            local band_settings = tostring(data:range(0, 10))

            local desc = string.format("set_equalizer(band_settings: %s)", band_settings)

            tree:add(pf_desc, desc)
        end
    end

    -- sidetone
    if feature == 0x07 then
        if fnid == 0x00 then
            tree:add(pf_desc, "get_sidetone_volume()")
        end
        if fnid == 0x01 then
            local volume = data:range(0, 1):uint()

            local desc = string.format("set_sidetone_volume(volume: %u)", volume)

            tree:add(pf_desc, desc)
        end
    end

    -- power
    if feature == 0x08 then
        if fnid == 0x00 then
            tree:add(pf_desc, "get_battery_status()")
        end
        if fnid == 0x01 then
            tree:add(pf_desc, "get_poweroff_timeout()")
        end
        if fnid == 0x02 then
            local timeout = data:range(0, 1):uint()

            local desc = string.format("set_poweroff_timeout(timeout: %u)", timeout)

            tree:add(pf_desc, desc)
        end
    end
end

local function dissector_response(tvbuf, pktinfo, root)
    local tree = dissector(tvbuf, pktinfo, root)

    local feature = tvbuf:range(2, 1):uint()
    local fnid = tvbuf:range(3, 1):bitfield(0, 4)
    local data = tvbuf:range(4, tvbuf:len() - 4)

    -- root
    if feature == 0x00 then
        if fnid == 0x00 then
            local index = data:range(0, 1):uint()
            local type_ = data:range(1, 1):uint()
            local version = data:range(2, 1):uint()

            local desc = string.format(
                "get_feature -> index: 0x%x, type: 0x%x, version: 0x%x",
                index, type_, version
            )

            tree:add(pf_desc, desc)
        end
        if fnid == 0x01 then
            local protocol_num = data:range(0, 1):uint()
            local target_sw = data:range(1, 1):uint()
            local ping_data = data:range(2, 1):uint()

            local desc = string.format(
                "get_protocol_version -> protocol_num: 0x%x, target_sw: 0x%x, ping_data: 0x%x",
                protocol_num, target_sw, ping_data
            )

            tree:add(pf_desc, desc)
        end
    end

    -- feature set
    if feature == 0x01 then
        if fnid == 0x00 then
            local count = data:range(0, 1):uint()

            local desc = string.format("get_feature_count -> count: 0x%x", count)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x01 then
            local id = data:range(0, 1):uint()

            local desc = string.format("get_feature_id -> id: 0x%x", id)

            tree:add(pf_desc, desc)
        end
    end

    -- device information
    if feature == 0x02 then
        if fnid == 0x00 then
            local entity_count = data:range(0, 1):uint()
            local unit_id = tostring(data:range(1, 4))
            local transport = tostring(data:range(5, 2))
            local model_id = tostring(data:range(7, 6))

            local desc = string.format(
                "get_device_info -> entity_count: 0x%x, unit_id: %s, transport: %s, model_id: %s",
                entity_count, unit_id, transport, model_id
            )

            tree:add(pf_desc, desc)
        end
        if fnid == 0x01 then
            local type_ = data:range(0, 1):uint()
            local fw_name = string.format(
                "%c%c%c%x",
                data:range(1, 1):uint(),
                data:range(2, 1):uint(),
                data:range(3, 1):uint(),
                data:range(4, 1):uint()
            )
            local rev = string.format("%x", data:range(5, 1):uint())
            local build = string.format("%x", data:range(6, 2):uint())
            local active = (data:range(8, 1):uint() == 0x01)
            local transport_pid = data:range(9, 2):uint()
            local extra_ver = tostring(data:range(11, 5))

            local desc = string.format(
                "get_fw_info -> type: 0x%x, fw_name: %s, rev: %s, build: %s, active: %s, trans_pid: 0x%x, ex_ver: %s",
                type_, fw_name, rev, build, active, transport_pid, extra_ver
            )

            tree:add(pf_desc, desc)
        end
    end

    -- device name/type
    if feature == 0x03 then
        if fnid == 0x00 then
            local length = data:range(0, 1):uint()

            local desc = string.format("get_device_name_length -> length: %u", length)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x01 then
            local name_part = data:range(0, 16):string()

            local desc = string.format("get_device_name -> name_part: %q", name_part)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x02 then
            local type_ = data:range(0, 1):uint()

            local desc = string.format("get_device_type -> type: 0x%x", type_)

            tree:add(pf_desc, desc)
        end
    end

    -- lights
    if feature == 0x04 then
        if fnid == 0x03 then
            local light, effect, save = dissect_lights(data)

            local desc = string.format("set_lights -> light: %s, effect: %s, save: %s", light, effect, save)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x04 then
            local enabled = data:range(0, 1):uint()

            local desc = string.format("get_startup_effect_enabled -> enabled: %s", enabled)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x05 then
            local unknown0 = data:range(0, 1):uint()
            local unknown1 = data:range(1, 1):uint()
            local enabled = data:range(2, 1):uint()

            local desc = string.format(
                "enable_startup_effect -> ??: 0x%x, ??: 0x%x, enabled: %s",
                unknown0, unknown1, enabled
            )

            tree:add(pf_desc, desc)
        end
    end

    -- gkeys
    if feature == 0x05 then
        if fnid == 0x00 then
            local count = data:range(0, 1):uint()

            local desc = string.format("get_button_count -> count: %u", count)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x01 then
            local enabled = (data:range(0, 1):uint() == 0x01)

            local desc = string.format("get_buttons_enabled -> enabled: %s", enabled)

            tree:add(pf_desc, desc)
        end
        if fnid == 2 then
            local enabled = (data:range(0, 1):uint() == 0x01)

            local desc = string.format("enable_buttons -> enabled: %s", enabled)

            tree:add(pf_desc, desc)
        end
    end

    -- equalizer
    if feature == 0x06 then
        if fnid == 0x00 then
            local num_bands = data:range(0, 1):uint()
            local band_range = data:range(1, 1):uint()
            local unknown0 = data:range(2, 1):uint()

            local desc = string.format(
                "get_equalizer_info -> num_bands: %u, band_range: %u, ??: 0x%x",
                num_bands, band_range, unknown0
            )

            tree:add(pf_desc, desc)
        end
        if fnid == 0x01 then
            local start_index = data:range(0, 1):uint()
            local bands = tostring(data:range(1, 14))

            local desc = string.format("get_equalizer_bands -> start_index: %u, bands: %s", start_index, bands)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x02 then
            local band_settings = tostring(data:range(0, 10))

            local desc = string.format("get_equalizer -> band_settings: %s", band_settings)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x03 then
            local band_settings = tostring(data:range(0, 10))

            local desc = string.format("set_equalizer -> band_settings: %s", band_settings)

            tree:add(pf_desc, desc)
        end
    end

    -- sidetone
    if feature == 0x07 then
        if fnid == 0x00 then
            local volume = data:range(0, 1):uint()

            local desc = string.format("get_sidetone_volume -> volume: %u", volume)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x01 then
            local volume = data:range(0, 1):uint()

            local desc = string.format("set_sidetone_volume -> volume: %u", volume)

            tree:add(pf_desc, desc)
        end
    end

    -- power
    if feature == 0x08 then
        if fnid == 0x00 then
            local charging_status_map = {
                [1] = "discharging",
                [3] = "charging",
                [7] = "full",
            }

            if tostring(data) == "00000000000000000000000000000000" then
                tree:add(pf_desc, "power_off")
            else
                local voltage = data:range(0, 2):uint()
                local status = charging_status_map[data:range(2, 1):uint()]

                local desc = string.format("get_battery_status -> voltage: %u, status: %s", voltage, status)

                tree:add(pf_desc, desc)
            end
        end
        if fnid == 0x01 then
            local timeout = data:range(0, 1):uint()

            local desc = string.format("get_poweroff_timeout -> timeout: %u", timeout)

            tree:add(pf_desc, desc)
        end
        if fnid == 0x02 then
            local timeout = data:range(0, 1):uint()

            local desc = string.format("set_poweroff_timeout -> timeout: %u", timeout)

            tree:add(pf_desc, desc)
        end
    end
end

local function heuristic(tvbuf)
    -- Checks the packet length and makes sure it's one of short, long, very_long
    if not (tvbuf:len() == 20) then
        return false
    end

    -- Checks against the first byte (length descriptor)
    if not (tvbuf:range(0, 1):uint() == 0x11) then
        return false
    end

    return true
end

local function heuristic_dissector_request(tvbuf, pktinfo, root)
    tvbuf = tvbuf:range(7, tvbuf:len() - 7):tvb()

    if not heuristic(tvbuf) then
        return false
    end

    -- Okay, it's our packet probably. Call the dissector.
    dissector_request(tvbuf, pktinfo, root)
    return true
end

local function heuristic_dissector_response(tvbuf, pktinfo, root)
    if not heuristic(tvbuf) then
        return false
    end

    -- Okay, it's our packet probably. Call the dissector.
    dissector_response(tvbuf, pktinfo, root)
    return true
end

hidpp:register_heuristic("usb.control", heuristic_dissector_request)
hidpp:register_heuristic("usb.interrupt", heuristic_dissector_response)
