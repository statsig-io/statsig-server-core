"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const bindings_1 = require("./bindings");
class StatsigUser {
    constructor(args) {
        this.__ref = (0, bindings_1.statsigUserCreate)(args.userID, JSON.stringify(args.customIDs), args.email, args.ip, args.userAgent, args.country, args.locale, args.appVersion, args.custom ? JSON.stringify(args.custom) : null, args.privateAttributes ? JSON.stringify(args.privateAttributes) : null);
    }
}
exports.default = StatsigUser;
