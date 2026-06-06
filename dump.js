const net = require('net');

const server = net.createServer((socket) => {
    socket.on('data', (data) => {
        console.log('--- RAW REQUEST DATA ---');
        console.log(data.toString('utf-8'));
        console.log('--- RAW HEX DATA ---');
        console.log(data.toString('hex'));
        
        // Send a valid HTTP response to close the connection happily
        const res = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
        socket.write(res);
        socket.end();
        
        console.log("Shutting down dumper...");
        process.exit(0);
    });
});

server.listen(3001, '0.0.0.0', () => {
    console.log('Listening on 0.0.0.0:3001 for device dump...');
});
