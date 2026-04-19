#!/usr/bin/env python3
"""
BACnet MS/TP - Discover device and read all objects
"""

import serial
import struct
import sys
import time

def calculate_crc8(data):
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x01:
                crc = (crc >> 1) ^ 0x81
            else:
                crc = crc >> 1
    return (~crc) & 0xFF

def calculate_crc16(data):
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x0001:
                crc = (crc >> 1) ^ 0x8408
            else:
                crc = crc >> 1
    return (~crc) & 0xFFFF

def create_mstp_frame(source_mac, dest_mac, npdu_apdu_data, expect_reply=False):
    """Create a complete MS/TP frame"""
    data_length = len(npdu_apdu_data)
    
    frame_type = 0x05 if expect_reply else 0x06
    
    header = bytes([
        0x55, 0xFF,  # Preamble
        frame_type,
        dest_mac,
        source_mac,
        (data_length >> 8) & 0xFF,
        data_length & 0xFF,
    ])
    
    header_crc = calculate_crc8(header[2:])
    data_crc = calculate_crc16(npdu_apdu_data)
    data_crc_bytes = struct.pack('<H', data_crc)
    
    return header + bytes([header_crc]) + npdu_apdu_data + data_crc_bytes

def create_whois():
    """Create Who-Is request"""
    npdu = bytes([0x01, 0x00])  # Version, no flags
    apdu = bytes([0x10, 0x08])  # Unconfirmed-REQ, Who-Is
    return npdu + apdu

def create_read_property(device_id, object_type, object_instance, property_id, invoke_id=1):
    """Create ReadProperty request"""
    npdu = bytes([0x01, 0x04])  # Version, expecting reply
    
    # APDU: Confirmed-REQ, ReadProperty
    apdu = bytes([
        0x00,  # Confirmed-REQ
        0x05,  # Max segments/max APDU (unspecified)
        invoke_id,
        0x0C,  # Service: ReadProperty
    ])
    
    # Object ID (context tag 0)
    obj_id = (object_type << 22) | object_instance
    apdu += bytes([
        0x0C,  # Context tag 0, length 4
        (obj_id >> 24) & 0xFF,
        (obj_id >> 16) & 0xFF,
        (obj_id >> 8) & 0xFF,
        obj_id & 0xFF,
    ])
    
    # Property ID (context tag 1)
    apdu += bytes([
        0x19,  # Context tag 1, length 1
        property_id,
    ])
    
    return npdu + apdu

def parse_complex_ack(data):
    """Parse ComplexACK response to extract property value"""
    try:
        # Skip MS/TP header (8 bytes)
        # Skip NPDU (2 bytes minimum)
        if len(data) < 12:
            return None
        
        npdu_start = 8
        npdu_version = data[npdu_start]
        npdu_control = data[npdu_start + 1]
        
        # Calculate NPDU length
        npdu_len = 2  # version + control
        has_dest = (npdu_control & 0x20) != 0
        has_src = (npdu_control & 0x08) != 0
        
        if has_dest:
            npdu_len += 2  # DNET
            dlen = data[npdu_start + npdu_len]
            npdu_len += 1 + dlen  # DLEN + DADR
            npdu_len += 1  # hop count
        
        if has_src:
            npdu_len += 2  # SNET
            slen = data[npdu_start + npdu_len]
            npdu_len += 1 + slen  # SLEN + SADR
        
        apdu_start = npdu_start + npdu_len
        if apdu_start >= len(data):
            return None
        
        apdu_type = (data[apdu_start] >> 4) & 0x0F
        
        if apdu_type != 3:  # Not ComplexACK
            return None
        
        # ComplexACK format: [PDU type/flags] [invoke ID] [service] [data...]
        invoke_id = data[apdu_start + 1]
        service = data[apdu_start + 2]
        
        if service != 0x0C:  # Not ReadProperty response
            return None
        
        # Parse property value (starts after service byte)
        value_start = apdu_start + 3
        
        # Skip context tags (object ID, property ID)
        idx = value_start
        while idx < len(data):
            tag = data[idx]
            
            # Opening tag for property value (context tag 3)
            if tag == 0x3E:
                idx += 1
                break
            
            # Skip context tags
            if (tag & 0x08) != 0:  # Context-specific tag
                tag_num = (tag >> 4) & 0x0F
                length = tag & 0x07
                
                if length == 5:  # Extended length
                    length = data[idx + 1]
                    idx += 2 + length
                else:
                    idx += 1 + length
            else:
                idx += 1
        
        # Now parse the application tag
        if idx >= len(data):
            return None
        
        tag = data[idx]
        tag_num = (tag >> 4) & 0x0F
        length = tag & 0x07
        
        if length == 5:  # Extended length
            length = data[idx + 1]
            value_start = idx + 2
        else:
            value_start = idx + 1
        
        value_data = data[value_start:value_start + length]
        
        # Decode based on tag type
        if tag_num == 0:  # Null
            return "null"
        elif tag_num == 1:  # Boolean
            return bool(length)
        elif tag_num == 2:  # Unsigned
            value = int.from_bytes(value_data, 'big')
            return value
        elif tag_num == 3:  # Signed
            value = int.from_bytes(value_data, 'big', signed=True)
            return value
        elif tag_num == 4:  # Real
            import struct
            return struct.unpack('>f', value_data)[0]
        elif tag_num == 5:  # Double
            import struct
            return struct.unpack('>d', value_data)[0]
        elif tag_num == 6:  # Octet String
            return value_data.hex(' ')
        elif tag_num == 7:  # Character String
            # First byte is encoding (0 = UTF-8)
            encoding = value_data[0]
            if encoding == 0:
                return value_data[1:].decode('utf-8', errors='replace')
            else:
                return value_data[1:].decode('latin-1', errors='replace')
        elif tag_num == 8:  # Bit String
            # First byte is unused bits count
            unused_bits = value_data[0]
            bits_data = value_data[1:]
            total_bits = len(bits_data) * 8 - unused_bits
            
            # Convert to binary string
            bit_str = ''.join(format(byte, '08b') for byte in bits_data)
            bit_str = bit_str[:total_bits]
            
            # Show as list of set bit positions
            set_bits = [i for i, bit in enumerate(bit_str) if bit == '1']
            return f"BitString({total_bits} bits): {set_bits}"
        elif tag_num == 9:  # Enumerated
            value = int.from_bytes(value_data, 'big')
            return f"Enum({value})"
        elif tag_num == 10:  # Date
            return f"Date: {value_data.hex(' ')}"
        elif tag_num == 11:  # Time
            return f"Time: {value_data.hex(' ')}"
        elif tag_num == 12:  # Object Identifier
            obj_id = int.from_bytes(value_data, 'big')
            obj_type = (obj_id >> 22) & 0x3FF
            instance = obj_id & 0x3FFFFF
            
            obj_types = {
                0: "AnalogInput", 1: "AnalogOutput", 2: "AnalogValue",
                3: "BinaryInput", 4: "BinaryOutput", 5: "BinaryValue",
                8: "Device", 13: "MultiStateInput", 14: "MultiStateOutput",
                19: "MultiStateValue", 48: "BitstringValue"
            }
            type_name = obj_types.get(obj_type, f"Type{obj_type}")
            return f"{type_name}:{instance}"
        else:
            return f"Unknown tag {tag_num}: {value_data.hex(' ')}"
        
    except Exception as e:
        return f"Parse error: {e}"

def parse_iam_response(data):
    """Parse I-Am response to extract device info
    
    I-Am format (Unconfirmed-REQ):
    - APDU Type: 0x10 (Unconfirmed-REQ)
    - Service: 0x00 (I-Am)
    - Device Object ID (application tag 12)
    - Max APDU Length (application tag 2)
    - Segmentation Support (application tag 9)
    - Vendor ID (application tag 2)
    """
    try:
        # MS/TP frame: [55 FF] [type] [dest] [src] [len_hi] [len_lo] [hdr_crc] [data...] [data_crc]
        # NPDU: [version] [control] [...]
        # APDU starts after NPDU
        
        if len(data) < 12:
            return None
        
        # MS/TP header is 8 bytes (including header CRC)
        npdu_start = 8
        npdu_version = data[npdu_start]
        npdu_control = data[npdu_start + 1]
        
        # Calculate NPDU length based on control flags
        npdu_len = 2  # version + control
        has_dest = (npdu_control & 0x20) != 0
        has_src = (npdu_control & 0x08) != 0
        
        if has_dest:
            npdu_len += 2  # DNET (2 bytes)
            dlen = data[npdu_start + npdu_len]
            npdu_len += 1 + dlen  # DLEN + DADR
            npdu_len += 1  # hop count
        
        if has_src:
            npdu_len += 2  # SNET (2 bytes)
            slen = data[npdu_start + npdu_len]
            npdu_len += 1 + slen  # SLEN + SADR
        
        apdu_start = npdu_start + npdu_len
        if apdu_start >= len(data) - 2:  # -2 for data CRC
            return None
        
        # Check APDU type
        apdu_type = (data[apdu_start] >> 4) & 0x0F
        if apdu_type != 1:  # Not Unconfirmed-REQ
            return None
        
        # Check service choice
        service = data[apdu_start + 1]
        if service != 0x00:  # Not I-Am
            return None
        
        # Parse parameters (all application tags)
        idx = apdu_start + 2
        
        # Device Object ID (application tag 12 - Object Identifier)
        tag = data[idx]
        tag_num = (tag >> 4) & 0x0F
        length = tag & 0x07
        
        if tag_num != 12 or length != 4:  # Must be Object ID with 4 bytes
            return None
        
        device_obj_id = struct.unpack('>I', data[idx+1:idx+5])[0]
        device_id = device_obj_id & 0x3FFFFF
        idx += 5
        
        # Max APDU Length (application tag 2 - Unsigned)
        tag = data[idx]
        tag_num = (tag >> 4) & 0x0F
        length = tag & 0x07
        if tag_num == 2:
            max_apdu = int.from_bytes(data[idx+1:idx+1+length], 'big')
            idx += 1 + length
        
        # Segmentation Support (application tag 9 - Enumerated)
        tag = data[idx]
        tag_num = (tag >> 4) & 0x0F
        length = tag & 0x07
        if tag_num == 9:
            segmentation = data[idx+1]
            idx += 1 + length
        
        # Vendor ID (application tag 2 - Unsigned)
        tag = data[idx]
        tag_num = (tag >> 4) & 0x0F
        length = tag & 0x07
        if tag_num == 2:
            vendor_id = int.from_bytes(data[idx+1:idx+1+length], 'big')
        
        return {
            'device_id': device_id,
            'source_mac': data[4],  # Source address from MS/TP header
        }
    except Exception as e:
        print(f"I-Am parse error: {e}")
        return None

def parse_object_list(data):
    """Parse object list from ComplexACK response
    
    Returns list of (object_type, instance) tuples
    """
    try:
        # Skip MS/TP header (8 bytes)
        if len(data) < 12:
            return None
        
        npdu_start = 8
        npdu_control = data[npdu_start + 1]
        
        # Calculate NPDU length
        npdu_len = 2  # version + control
        has_dest = (npdu_control & 0x20) != 0
        has_src = (npdu_control & 0x08) != 0
        
        if has_dest:
            npdu_len += 2  # DNET
            dlen = data[npdu_start + npdu_len]
            npdu_len += 1 + dlen  # DLEN + DADR
            npdu_len += 1  # hop count
        
        if has_src:
            npdu_len += 2  # SNET
            slen = data[npdu_start + npdu_len]
            npdu_len += 1 + slen  # SLEN + SADR
        
        apdu_start = npdu_start + npdu_len
        if apdu_start >= len(data):
            return None
        
        apdu_type = (data[apdu_start] >> 4) & 0x0F
        if apdu_type != 3:  # Not ComplexACK
            return None
        
        # Skip to property value (after opening tag 0x3E)
        idx = apdu_start + 3
        while idx < len(data) - 2:  # -2 for data CRC
            if data[idx] == 0x3E:  # Opening tag for property value
                idx += 1
                break
            idx += 1
        
        # Parse object identifiers
        objects = []
        obj_types = {
            0: "AnalogInput", 1: "AnalogOutput", 2: "AnalogValue",
            3: "BinaryInput", 4: "BinaryOutput", 5: "BinaryValue",
            8: "Device", 13: "MultiStateInput", 14: "MultiStateOutput",
            19: "MultiStateValue", 48: "BitstringValue"
        }
        
        while idx < len(data) - 2:  # -2 for data CRC
            tag = data[idx]
            
            # Check for closing tag (0x3F)
            if tag == 0x3F:
                break
            
            # Parse application tag 12 (Object Identifier)
            tag_num = (tag >> 4) & 0x0F
            length = tag & 0x07
            
            if tag_num == 12 and length == 4:
                obj_id = int.from_bytes(data[idx+1:idx+5], 'big')
                obj_type = (obj_id >> 22) & 0x3FF
                instance = obj_id & 0x3FFFFF
                
                type_name = obj_types.get(obj_type, f"Type{obj_type}")
                objects.append((type_name, instance))
                idx += 5
            else:
                idx += 1
        
        return objects
        
    except Exception as e:
        print(f"Object list parse error: {e}")
        return None

def send_and_receive(ser, frame, timeout=2):
    """Send frame and wait for response"""
    ser.write(frame)
    ser.flush()
    
    ser.timeout = timeout
    response = ser.read(1024)
    return response if response else None

def main():
    PORT = '/dev/ttyUSB0'
    BAUDRATE = 115200
    SOURCE_MAC = 5
    
    try:
        ser = serial.Serial(
            port=PORT,
            baudrate=BAUDRATE,
            bytesize=serial.EIGHTBITS,
            parity=serial.PARITY_NONE,
            stopbits=serial.STOPBITS_ONE,
            timeout=1,
        )
        
        print(f"BACnet MS/TP Object Reader")
        print(f"Port: {PORT} @ {BAUDRATE} baud, Source MAC: {SOURCE_MAC}\n")
        
        # Step 1: Send Who-Is
        print("Sending Who-Is...")
        whois_data = create_whois()
        whois_frame = create_mstp_frame(SOURCE_MAC, 255, whois_data, expect_reply=False)
        response = send_and_receive(ser, whois_frame, timeout=3)
        
        if not response:
            print("No I-Am response received")
            return
        
        # Parse I-Am
        device_info = parse_iam_response(response)
        if not device_info:
            print("Failed to parse I-Am response")
            print(f"Raw response: {response.hex(' ')}")
            return
        
        print(f"✓ Found device {device_info['device_id']} at MAC {device_info['source_mac']}\n")
        
        dest_mac = device_info['source_mac']
        device_id = device_info['device_id']
        
        # Step 2: Read device object list (property 76)
        print("Reading object list...")
        read_obj_list = create_read_property(device_id, 8, device_id, 76, invoke_id=1)
        frame = create_mstp_frame(SOURCE_MAC, dest_mac, read_obj_list, expect_reply=True)
        response = send_and_receive(ser, frame, timeout=2)
        
        if response:
            objects = parse_object_list(response)
            if objects:
                print(f"✓ Found {len(objects)} objects:")
                for obj_type, instance in objects:
                    print(f"  {obj_type}:{instance}")
                print()
            else:
                print(f"Failed to parse object list ({len(response)} bytes)")
                print(f"  Raw: {response.hex(' ')}\n")
        else:
            print("No response to object list request\n")
        
        # Step 3: Try reading some common properties
        print("Reading device properties...")
        
        properties = [
            (77, "Object Name"),
            (70, "Object Type"),
            (75, "Object Identifier"),
            (120, "Vendor Name"),
            (121, "Vendor Identifier"),
            (70, "Model Name"),
        ]
        
        for prop_id, prop_name in properties:
            time.sleep(0.1)  # Small delay between requests
            read_prop = create_read_property(device_id, 8, device_id, prop_id, invoke_id=prop_id)
            frame = create_mstp_frame(SOURCE_MAC, dest_mac, read_prop, expect_reply=True)
            response = send_and_receive(ser, frame, timeout=2)
            
            if response:
                value = parse_complex_ack(response)
                if value:
                    print(f"  {prop_name} (ID {prop_id}): {value}")
                else:
                    print(f"  {prop_name} (ID {prop_id}): {len(response)} bytes - {response.hex(' ')}")
            else:
                print(f"  {prop_name} (ID {prop_id}): No response")
        
        ser.close()
        
    except serial.SerialException as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()
