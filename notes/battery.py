import csv, subprocess
from collections import OrderedDict
from heapq import nsmallest

# A map of voltages in mV to SoCs
soc_map = {
    # Discharging voltage mappings
    1: OrderedDict((int(x), float(y)) for (x, y) in csv.reader(open("libg933/src/discharging.csv"))),

    # Charging voltage mappings
    # TODO: possibly monitor ascending/descending status to get proper estimates
    3: OrderedDict((int(x), float(y)) for (x, y) in csv.reader(open("libg933/src/charging_ascending.csv"))),
    #?: OrderedDict((float(x), float(y)) for (x, y) in csv.reader(open("libg933/src/charging_descending.csv"))),

    # Full voltage mappings
    7: OrderedDict(((0, 100), (1, 100))), # Hack to return 100% for "full" status
}

# Execute the command to retrieve battery info
b = subprocess.Popen(["target/release/g933-utils", "raw", "11", "ff", "08", "00"], stdout = subprocess.PIPE).stdout.read().split(b" ")
# Get voltage as a float in volts
voltage = int(b"".join(b[4:6]), 16)
print("voltage:", voltage)

# Get charging state as an int
state = int(b[6])
print("state:", state)

# Grab the two mapped voltages closest to the voltage we read, and sort them
closest_voltages = sorted(nsmallest(2, soc_map[state], key = lambda x: abs(x - voltage)))
print("closest voltages:", closest_voltages)

# Get the ratio (where the voltage appears in relation to the two closest mapped voltages)
ratio = (voltage - closest_voltages[0]) / (closest_voltages[1] - closest_voltages[0])
print("ratio:", ratio)

# Get the SoCs corresponding to the mapped voltages
closest_socs = [soc_map[state][closest_voltages[0]], soc_map[state][closest_voltages[1]]]
print("closest socs:", closest_socs)

# Calculate an estimate of what our actual SoC is based on the ratio of voltages
estimated_soc = ((closest_socs[1] - closest_socs[0]) * ratio) + closest_socs[0]
# Clamp result to 0-100 range
estimated_soc = max(0, min(100, estimated_soc))
print("estimated soc:", estimated_soc)
