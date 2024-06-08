# Walkie-Talkie App 
The Walkie-Talkie App is a mobile application designed for both iOS and Android platforms, allowing users to communicate over a local network. The app enables real-time audio communication among multiple users, with one user designated as the Admin who has the ability to mute all other devices. 

### Overview of the Alternative Approach

1. **Peer-to-Peer (P2P) Communication**: Enables direct communication between devices on the same local network.
2. **Service Discovery**: Implements a mechanism to discover other devices running the same application on the network.
3. **Identity Management**: Assigns unique identities to each instance of the application for recognition and communication.
4. **Group Management**: Allows users to create and manage groups locally on their devices.
5. **Data Broadcasting**: Enables broadcasting messages or data to specific groups or all devices on the network.

### Technologies and Tools

1. **Flutter**: For building the cross-platform user interface.
2. **Rust**: For implementing the core logic, including networking and P2P communication.
3. **Multicast DNS (mDNS) / DNS Service Discovery (DNS-SD)**: For service discovery on the local network.
4. **WebRTC or similar**: For establishing P2P connections and real-time communication.
5. **Local Database (SQLite)**: For storing user-created groups and settings.

### Example Workflow

1. **User Opens the App**:
   - The app uses mDNS/DNS-SD to discover other devices on the network running the same app.
   - Discovered devices are displayed in a list.

2. **User Creates Groups**:
   - The user can create groups and assign discovered devices to these groups.
   - Groups and assignments are stored locally on the device.

3. **User Sends Data**:
   - The user selects a group or the entire network to broadcast a message or audio.
   - The app uses WebRTC to establish P2P connections with the selected devices and sends the data.

### Admin Functionality

1. **Admin Designation**: Allow one device to be designated as the Admin.
2. **Admin Functions**: Implement specific functions that only the Admin can execute, such as muting all devices.
3. **Broadcast Control Commands**: The Admin device can broadcast control commands to other devices on the network.
4. **Device Response**: Other devices recognize and respond to these control commands.

### Example Workflow

1. **Admin Device Setup**
   - The user designates one device as the Admin.
   - The Admin device has additional controls in the UI for managing other devices.

2. **Control Command Broadcast**
   - When the Admin decides to mute all devices, they send a command using WebRTC data channels.
   - Example command: `{"action": "mute", "target": "all"}`.

3. **Receiving and Handling Commands**
   - All devices receive the command through WebRTC data channels.
   - Devices parse the command and execute the corresponding action, e.g., muting their audio.

### Conclusion

The Walkie-Talkie App aims to provide a reliable and efficient method for real-time audio communication over a local network. By leveraging P2P communication, service discovery, and admin-controlled features, the app ensures a seamless user experience without relying on an internet connection. The outlined development process and technologies provide a robust foundation for building and expanding the application's capabilities.
