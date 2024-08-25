import * as message from "./message.js";
import * as rendezvous from "./rendezvous.js";
import * as globals from "./globals";

type Keys = "message" | "open" | "close" | "error";

export default class Rtcsock {
  _peerConnection: RTCPeerConnection;
  _dataChannel: RTCDataChannel;
  _eventHandlers: { [key in Keys]: Function };
  _buf: (rendezvous.RendezvousMessage | message.Message)[];
  _status: any;
  _latency: number;
  _secretKey: [Uint8Array, number, number] | undefined;
  _uri: string;
  _isRendezvous: boolean;

  constructor(uri: string, isRendezvous: boolean = true) {
    this._eventHandlers = {
      message: (_: any) => { },
      open: () => { },
      close: () => { },
      error: () => { },
    };
    this._uri = uri;
    this._status = "";
    this._buf = [];
    const servers = {
      iceServers: [
        {
          urls: 'stun:stun.l.google.com:19302'
        }
      ]
    };;
    this._peerConnection = new RTCPeerConnection(servers);
    // this._peerConnection.oniceconnectionstatechange = this._oniceconnectionstatechange;
    this._dataChannel = this._peerConnection.createDataChannel("sendDataChannel");
    this._dataChannel.onmessage = this._recv_message.bind(this);
    this._dataChannel.binaryType = "arraybuffer";
    this._latency = new Date().getTime();
    this._isRendezvous = isRendezvous;
  }

  latency(): number {
    return this._latency;
  }

  setSecretKey(key: Uint8Array) {
    this._secretKey = [key, 0, 0];
  }

  sendMessage(json: message.DeepPartial<message.Message>) {
    let data = message.Message.encode(
      message.Message.fromPartial(json)
    ).finish();
    let k = this._secretKey;
    if (k) {
      k[1] += 1;
      data = globals.encrypt(data, k[1], k[0]);
    }
    this._dataChannel.send(data);
  }

  sendRendezvous(data: rendezvous.DeepPartial<rendezvous.RendezvousMessage>) {
    console.log("sendRendezvous:", data);
    this._dataChannel.send(
      rendezvous.RendezvousMessage.encode(
        rendezvous.RendezvousMessage.fromPartial(data)
      ).finish()
    );
    console.log("sendRendezvous: done");
  }

  parseMessage(data: Uint8Array) {
    return message.Message.decode(data);
  }

  parseRendezvous(data: Uint8Array) {
    return rendezvous.RendezvousMessage.decode(data);
  }

  // Event Handlers
  off(evt: Keys) {
    this._eventHandlers[evt] = () => { };
  }

  on(evt: Keys, handler: Function) {
    this._eventHandlers[evt] = handler;
  }

  // async _oniceconnectionstatechange(e: Event) {
  //       console.log("oniceconnectionstatechange:", this._peerConnection.iceConnectionState, "e:", e);
  // }


  async open(timeout: number = 12000): Promise<Rtcsock> {
    const offer = await this._peerConnection.createOffer()
    console.log(`Got local offer ${JSON.stringify(offer)}`);
    await this._peerConnection.setLocalDescription(offer);
    console.log("uri:", this._uri);
    try {
      const response = await fetch(`http://127.0.0.1:21114/api/rtc/signal/sdp`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer OS6zK0TCSSuPcVUk49MnBw=='
        },
        body: JSON.stringify({
          sdp: JSON.stringify(offer),
          rendezvous: this._isRendezvous,
          peer_id: ''
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result: RTCSessionDescriptionInit = await response.json();
      console.log('Server response:', result);
      await this._peerConnection.setRemoteDescription(result);
    } catch (error) {
      console.error('Failed to send localDesc:', error);
    }

    return new Promise((resolve, reject) => {
      setTimeout(() => {
        if (this._status != "open") {
          reject(this._status || "Timeout");
        }
      }, timeout);
      this._dataChannel.onopen = () => {
        this._latency = new Date().getTime() - this._latency;
        this._status = "open";
        console.debug(">> WebSock.onopen");
        if (this._dataChannel?.protocol) {
          console.info(
            "Server choose sub-protocol: " + this._dataChannel.protocol
          );
        }

        this._eventHandlers.open();
        console.info("WebSock.onopen");
        resolve(this);
      };
      this._dataChannel.onclose = (e) => {
        if (this._status == "open") {
          // e.code 1000 means that the connection was closed normally.
          //
        }
        this._status = e;
        console.error("WebSock.onclose: ");
        console.error(e);
        this._eventHandlers.close(e);
        reject("Reset by the peer");
      };
      this._dataChannel.onerror = (e: any) => {
        if (!this._status) {
          reject("Failed to connect to " + (this._isRendezvous ? "rendezvous" : "relay") + " server");
          return;
        }
        this._status = e;
        console.error("WebSock.onerror: ")
        console.error(e);
        this._eventHandlers.error(e);
      };
    });
  }

  async next(
    timeout = 12000
  ): Promise<rendezvous.RendezvousMessage | message.Message> {
    const func = (
      resolve: (value: rendezvous.RendezvousMessage | message.Message) => void,
      reject: (reason: any) => void,
      tm0: number
    ) => {
      if (this._buf.length) {
        resolve(this._buf[0]);
        this._buf.splice(0, 1);
      } else {
        if (this._status != "open") {
          reject(this._status);
          return;
        }
        if (new Date().getTime() > tm0 + timeout) {
          reject("Timeout");
        } else {
          setTimeout(() => func(resolve, reject, tm0), 1);
        }
      }
    };
    return new Promise((resolve, reject) => {
      func(resolve, reject, new Date().getTime());
    });
  }

  close() {
    this._status = "";
    if (this._peerConnection) {
      console.info("Closing WebSocket connection");
      this._peerConnection.close();
      this._dataChannel.close();
      this._dataChannel.onmessage = () => { };
    }
  }

  _recv_message(e: any) {
    console.log("======= recv_message:", e);
    if (e.data instanceof window.ArrayBuffer) {
      let bytes = new Uint8Array(e.data);
      const k = this._secretKey;
      if (k) {
        k[2] += 1;
        bytes = globals.decrypt(bytes, k[2], k[0]);
      }
      this._buf.push(
        this._isRendezvous
          ? this.parseRendezvous(bytes)
          : this.parseMessage(bytes)
      );
    }
    this._eventHandlers.message(e.data);
  }
}
