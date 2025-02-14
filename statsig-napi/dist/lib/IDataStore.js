"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CompressFormat = exports.DataStoreKeyPath = void 0;
exports.getDataStoreKey = getDataStoreKey;
const STATSIG_PREFIX = 'statsig';
var DataStoreKeyPath;
(function (DataStoreKeyPath) {
    DataStoreKeyPath["V1Rulesets"] = "/v1/download_config_specs";
    DataStoreKeyPath["V2Rulesets"] = "/v2/download_config_specs";
    DataStoreKeyPath["V1IDLists"] = "/v1/get_id_lists";
    DataStoreKeyPath["IDList"] = "id_list";
})(DataStoreKeyPath || (exports.DataStoreKeyPath = DataStoreKeyPath = {}));
var CompressFormat;
(function (CompressFormat) {
    CompressFormat["PlainText"] = "plain_text";
    CompressFormat["Gzip"] = "gzip";
})(CompressFormat || (exports.CompressFormat = CompressFormat = {}));
function getDataStoreKey(hashedSDKKey, path, format = CompressFormat.PlainText, idListName = undefined) {
    if (path == DataStoreKeyPath.IDList) {
        return `${STATSIG_PREFIX}|${path}::${String(idListName)}|${format}|${hashedSDKKey}`;
    }
    else {
        return `${STATSIG_PREFIX}|${path}|${format}|${hashedSDKKey}`;
    }
}
