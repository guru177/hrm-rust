const net = require('net');
const fs = require('fs');

const server = net.createServer((socket) => {
    socket.on('data', (data) => {
        if (data.length >= 8 && data[0] === 0xA5 && data[1] === 0x5A) {
            
            const commandId = data.readUInt16LE(2); // Wait, if A5 5A is first 2, then cmd is next 2.
            
            if (commandId === 1) { // CONNECT
                console.log("[DATA] Received Handshake. Sending custom ACK & REG_EVENT...");
                
                // Construct ACK (A5 5A D0 07 + 4 bytes of session data)
                const ack = Buffer.alloc(8);
                ack[0] = 0xA5; ack[1] = 0x5A;
                ack[2] = 0xD0; ack[3] = 0x07;
                ack[4] = data[4]; ack[5] = data[5]; ack[6] = data[6]; ack[7] = data[7];
                socket.write(ack);
                
                // Construct CMD_REG_EVENT (500 = F4 01)
                const regEvent = Buffer.alloc(12);
                regEvent[0] = 0xA5; regEvent[1] = 0x5A;
                regEvent[2] = 0xF4; regEvent[3] = 0x01; // CMD = 500
                regEvent[4] = data[4]; regEvent[5] = data[5]; regEvent[6] = data[6]; regEvent[7] = data[7];
                // Payload
                regEvent[8] = 0x01; regEvent[9] = 0x00; regEvent[10] = 0x00; regEvent[11] = 0x00;
                socket.write(regEvent);
                
            } else {
                console.log("💥 [DATA] Received SOMETHING ELSE:");
                console.log("Command ID:", commandId);
                console.log(data.toString('hex'));
                
                // ACK the event so it doesn't loop
                const ack = Buffer.alloc(8);
                ack[0] = 0xA5; ack[1] = 0x5A;
                ack[2] = 0xD0; ack[3] = 0x07;
                ack[4] = data[4]; ack[5] = data[5]; ack[6] = data[6]; ack[7] = data[7];
                socket.write(ack);
            }
        }
    });

    socket.on('error', () => {});
});

server.listen(3001, '0.0.0.0', () => {
    console.log('Listening for device gently on 0.0.0.0:3001...');
});
