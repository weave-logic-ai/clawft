"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.containerDetector = exports.ContainerDetector = void 0;
/*
 * Copyright The OpenTelemetry Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
const resources_1 = require("@opentelemetry/resources");
const semantic_conventions_1 = require("@opentelemetry/semantic-conventions");
const fs = require("fs");
const util = require("util");
const api_1 = require("@opentelemetry/api");
class ContainerDetector {
    constructor() {
        this.CONTAINER_ID_LENGTH = 64;
        this.DEFAULT_CGROUP_V1_PATH = '/proc/self/cgroup';
        this.DEFAULT_CGROUP_V2_PATH = '/proc/self/mountinfo';
        this.UTF8_UNICODE = 'utf8';
        this.HOSTNAME = 'hostname';
    }
    async detect(_config) {
        try {
            const containerId = await this._getContainerId();
            return !containerId
                ? resources_1.Resource.empty()
                : new resources_1.Resource({
                    [semantic_conventions_1.SEMRESATTRS_CONTAINER_ID]: containerId,
                });
        }
        catch (e) {
            api_1.diag.info('Container Detector did not identify running inside a supported container, no container attributes will be added to resource: ', e);
            return resources_1.Resource.empty();
        }
    }
    async _getContainerIdV1() {
        const rawData = await ContainerDetector.readFileAsync(this.DEFAULT_CGROUP_V1_PATH, this.UTF8_UNICODE);
        const splitData = rawData.trim().split('\n');
        for (const line of splitData) {
            const lastSlashIdx = line.lastIndexOf('/');
            if (lastSlashIdx === -1) {
                continue;
            }
            const lastSection = line.substring(lastSlashIdx + 1);
            const colonIdx = lastSection.lastIndexOf(':');
            if (colonIdx !== -1) {
                // since containerd v1.5.0+, containerId is divided by the last colon when the cgroupDriver is systemd:
                // https://github.com/containerd/containerd/blob/release/1.5/pkg/cri/server/helpers_linux.go#L64
                return lastSection.substring(colonIdx + 1);
            }
            else {
                let startIdx = lastSection.lastIndexOf('-');
                let endIdx = lastSection.lastIndexOf('.');
                startIdx = startIdx === -1 ? 0 : startIdx + 1;
                if (endIdx === -1) {
                    endIdx = lastSection.length;
                }
                if (startIdx > endIdx) {
                    continue;
                }
                return lastSection.substring(startIdx, endIdx);
            }
        }
        return undefined;
    }
    async _getContainerIdV2() {
        const rawData = await ContainerDetector.readFileAsync(this.DEFAULT_CGROUP_V2_PATH, this.UTF8_UNICODE);
        const str = rawData
            .trim()
            .split('\n')
            .find(s => s.includes(this.HOSTNAME));
        const containerIdStr = str === null || str === void 0 ? void 0 : str.split('/').find(s => s.length === this.CONTAINER_ID_LENGTH);
        return containerIdStr || '';
    }
    /*
      cgroupv1 path would still exist in case of container running on v2
      but the cgroupv1 path would no longer have the container id and would
      fallback on the cgroupv2 implementation.
    */
    async _getContainerId() {
        try {
            return ((await this._getContainerIdV1()) || (await this._getContainerIdV2()));
        }
        catch (e) {
            if (e instanceof Error) {
                const errorMessage = e.message;
                api_1.diag.info('Container Detector failed to read the Container ID: ', errorMessage);
            }
        }
        return undefined;
    }
}
exports.ContainerDetector = ContainerDetector;
ContainerDetector.readFileAsync = util.promisify(fs.readFile);
exports.containerDetector = new ContainerDetector();
//# sourceMappingURL=ContainerDetector.js.map