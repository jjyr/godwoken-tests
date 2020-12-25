import { ChainService } from "@ckb-godwoken/godwoken";
import jayson from "jayson/promise";
import cors from "cors";
import connect from "connect";
import { json } from "body-parser";

function isHexString(s: any) {
  return typeof s === "string" && /^0x([0-9a-fA-F][0-9a-fA-F])*$/.test(s);
}

function isHash(s: any) {
  return isHexString(s) && s.length === 66;
}

export class JsonrpcServer {
  chainService: ChainService;
  server: jayson.Server;
  listen: string;

  constructor(chainService: ChainService, listen: string) {
    this.chainService = chainService;
    this.server = new jayson.Server({
      ckb_submitL2Transaction: this.submitL2Transaction.bind(this),
      ckb_executeL2Tranaction: this.executeL2Transaction.bind(this),
      ckb_submitWithdrawalRequest: this.submitWithdrawalRequest.bind(this),
      ckb_getBalance: this.getBalance.bind(this),
      ckb_getStorageAt: this.getStorageAt.bind(this),
      ckb_getAccountIdByScriptHash: this.getAccountIdByScriptHash.bind(this),
    });
    this.listen = listen;
  }

  async submitL2Transaction(args: any) {
    if (args.length !== 1 || !isHexString(args[0])) {
      throw this.server.error(501, "Invalid arguments!");
    }
    return await this.chainService.submitL2Transaction(args[0]);
  }

  async executeL2Transaction(args: any) {
    if (args.length !== 1 || !isHexString(args[0])) {
      throw this.server.error(501, "Invalid arguments!");
    }
    return await this.chainService.execute(args[0]);
  }

  async submitWithdrawalRequest(args: any) {
    if (args.length !== 1 || !isHexString(args[0])) {
      throw this.server.error(501, "Invalid arguments!");
    }
    await this.chainService.submitWithdrawalRequest(args[0]);
    return "OK";
  }

  async getBalance(args: any) {
    if (
      args.length !== 2 ||
      typeof args[0] !== "number" ||
      typeof args[1] !== "number"
    ) {
      throw this.server.error(501, "Invalid arguments!");
    }
    return await this.chainService.getBalance(args[0], args[1]);
  }

  async getStorageAt(args: any) {
    if (args.length !== 2 || typeof args[0] !== "number" || !isHash(args[1])) {
      throw this.server.error(501, "Invalid arguments!");
    }
    return await this.chainService.getStorageAt(args[0], args[1]);
  }

  async getAccountIdByScriptHash(args: any) {
    if (args.length !== 1 || !isHash(args[0])) {
      throw this.server.error(501, "Invalid arguments!");
    }
    return await this.chainService.getAccountIdByScriptHash(args[0]);
  }

  async start() {
    const app = connect();
    app.use(cors({ methods: ["GET", "PUT", "POST"] }));
    app.use(json());
    app.use(this.server.middleware());

    app.listen(this.listen);
  }
}
