const { existsSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

let nativeBinding = null
let localFileExisted = false
let loadError = null

function isMusl() {
  const { glibcVersionRuntime } = process.report.getReport().header
  return !glibcVersionRuntime
}

switch (platform) {
  case 'android':
    switch (arch) {
      case 'arm64':
        localFileExisted = existsSync(join(__dirname, 'rspack.android-arm64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./rspack.android-arm64.node')
          } else {
            nativeBinding = require('@rspack/binding-testing-android-arm64')
          }
        } catch (e) {
          loadError = e
        }
        break
      case 'arm':
        localFileExisted = existsSync(join(__dirname, 'rspack.android-arm-eabi.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./rspack.android-arm-eabi.node')
          } else {
            nativeBinding = require('@rspack/binding-testing-android-arm-eabi')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on Android ${arch}`)
    }
    break
  case 'win32':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(join(__dirname, 'rspack.win32-x64-msvc.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./rspack.win32-x64-msvc.node')
          } else {
            nativeBinding = require('@rspack/binding-testing-win32-x64-msvc')
          }
        } catch (e) {
          loadError = e
        }
        break
      case 'ia32':
        localFileExisted = existsSync(join(__dirname, 'rspack.win32-ia32-msvc.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./rspack.win32-ia32-msvc.node')
          } else {
            nativeBinding = require('@rspack/binding-testing-win32-ia32-msvc')
          }
        } catch (e) {
          loadError = e
        }
        break
      case 'arm64':
        localFileExisted = existsSync(join(__dirname, 'rspack.win32-arm64-msvc.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./rspack.win32-arm64-msvc.node')
          } else {
            nativeBinding = require('@rspack/binding-testing-win32-arm64-msvc')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on Windows: ${arch}`)
    }
    break
  case 'darwin':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(join(__dirname, 'rspack.darwin-x64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./rspack.darwin-x64.node')
          } else {
            nativeBinding = require('@rspack/binding-testing-darwin-x64')
          }
        } catch (e) {
          loadError = e
        }
        break
      case 'arm64':
        localFileExisted = existsSync(join(__dirname, 'rspack.darwin-arm64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./rspack.darwin-arm64.node')
          } else {
            nativeBinding = require('@rspack/binding-testing-darwin-arm64')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on macOS: ${arch}`)
    }
    break
  case 'freebsd':
    if (arch !== 'x64') {
      throw new Error(`Unsupported architecture on FreeBSD: ${arch}`)
    }
    localFileExisted = existsSync(join(__dirname, 'rspack.freebsd-x64.node'))
    try {
      if (localFileExisted) {
        nativeBinding = require('./rspack.freebsd-x64.node')
      } else {
        nativeBinding = require('@rspack/binding-testing-freebsd-x64')
      }
    } catch (e) {
      loadError = e
    }
    break
  case 'linux':
    switch (arch) {
      case 'x64':
        if (isMusl()) {
          localFileExisted = existsSync(join(__dirname, 'rspack.linux-x64-musl.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require('./rspack.linux-x64-musl.node')
            } else {
              nativeBinding = require('@rspack/binding-testing-linux-x64-musl')
            }
          } catch (e) {
            loadError = e
          }
        } else {
          localFileExisted = existsSync(join(__dirname, 'rspack.linux-x64-gnu.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require('./rspack.linux-x64-gnu.node')
            } else {
              nativeBinding = require('@rspack/binding-testing-linux-x64-gnu')
            }
          } catch (e) {
            loadError = e
          }
        }
        break
      case 'arm64':
        if (isMusl()) {
          localFileExisted = existsSync(join(__dirname, 'rspack.linux-arm64-musl.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require('./rspack.linux-arm64-musl.node')
            } else {
              nativeBinding = require('@rspack/binding-testing-linux-arm64-musl')
            }
          } catch (e) {
            loadError = e
          }
        } else {
          localFileExisted = existsSync(join(__dirname, 'rspack.linux-arm64-gnu.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require('./rspack.linux-arm64-gnu.node')
            } else {
              nativeBinding = require('@rspack/binding-testing-linux-arm64-gnu')
            }
          } catch (e) {
            loadError = e
          }
        }
        break
      case 'arm':
        localFileExisted = existsSync(join(__dirname, 'rspack.linux-arm-gnueabihf.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./rspack.linux-arm-gnueabihf.node')
          } else {
            nativeBinding = require('@rspack/binding-testing-linux-arm-gnueabihf')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on Linux: ${arch}`)
    }
    break
  default:
    throw new Error(`Unsupported OS: ${platform}, architecture: ${arch}`)
}

if (!nativeBinding || process.env.NAPI_RS_FORCE_WASI) {
  try {
    nativeBinding = require('./rspack.wasi.cjs')
  } catch (e) {
    if (process.env.NAPI_RS_FORCE_WASI) {
      loadError = e
    }
  }
  if (!nativeBinding) {
    try {
      nativeBinding = require('@rspack/binding-testing-wasm32-wasi')
    } catch (e) {
      if (process.env.NAPI_RS_FORCE_WASI) {
        loadError = e
      }
    }
  }
}

if (!nativeBinding) {
  if (loadError) {
    throw loadError
  }
  throw new Error(`Failed to load native binding`)
}

module.exports.default = module.exports = nativeBinding
