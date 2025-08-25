/* eslint-disable no-irregular-whitespace */
/* eslint-disable max-len */
import type { BaseTranslation } from '../i18n-types';

const en = {
  time: {
    seconds: {
      singular: 'second',
      plural: 'seconds',
    },
    minutes: {
      singular: 'minute',
      plural: 'minutes',
    },
  },
  form: {
    errors: {
      invalid: 'Field is invalid',
      email: 'Enter a valid E-mail',
      required: 'Field is required',
      minValue: 'Field requires minimal value of {min: number}',
      maxValue: 'Field cannot exceed maximal value of {max: number}',
      aboveZero: 'Field value must be above zero',
      minLength: 'Min length of {length: number}',
      maxLength: 'Max length of {length: number}',
      specialsRequired: 'At least one special character',
      specialsForbidden: 'Special characters are forbidden',
      numberRequired: 'At least one number required',
      password: {
        floatingTitle: 'Please correct the following:',
      },
      oneLower: 'At least one lower case character',
      oneUpper: 'At least one upper case character',
      duplicatedName: 'Device with this name already exists',
    },
  },
  common: {
    controls: {
      back: 'Back',
      next: 'Next',
      submit: 'Submit',
      cancel: 'Cancel',
      close: 'Close',
      reset: 'Reset',
      save: 'Save',
    },
    messages: {
      error: 'Unexpected error occurred!',
      errorWithMessage: 'An error occurred: {message}',
      tokenExpired:
        'Token has expired, please contact your administrator to issue a new enrollment token',
      networkError: "There was a network error. Can't reach proxy.",
      configChanged:
        'Configuration for instance {instance: string} has changed. Disconnect from all locations to apply changes.',
      deadConDropped:
        'Detected that the {con_type: string} {interface_name: string} has disconnected, trying to reconnect...',
      noCookie: 'No defguard_proxy set-cookie received',
      insecureContext: 'Context is not secure.',
      clipboard: {
        error: 'Clipboard is not accessible.',
        success: 'Content copied to clipboard.',
      },
    },
  },
  components: {
    adminInfo: {
      title: 'Your admin',
    },
  },
  pages: {
    client: {
      modals: {
        deadConDropped: {
          title: '{conType: string} {name: string} disconnected',
          tunnel: 'Tunnel',
          location: 'Location',
          message:
            'The {conType: string} {name: string} has been disconnected, since we have detected that the server is not responding with any traffic for {time: number}s. If this message keeps occurring, please contact your administrator and inform them about this fact.',
          controls: {
            close: 'Close',
          },
        },
      },
      pages: {
        carouselPage: {
          slides: {
            shared: {
              //md
              isMore: '**defguard** is all the above and more!',
              githubButton: 'Visit defguard on',
            },
            welcome: {
              // md
              title: 'Welcome to **defguard** desktop client!',
              instance: {
                title: 'Add Instance',
                subtitle:
                  'Establish a connection to defguard instance effortlessly by configuring it with a single token.',
              },
              tunnel: {
                title: 'Add Tunnel',
                subtitle:
                  'Utilize it as a WireGuard® Desktop Client with ease. Set up your own tunnel or import a configuration file.',
              },
            },
            twoFa: {
              // md
              title: 'WireGuard **2FA with defguard**',
              // md
              sideText: `Since WireGuard protocol doesn't support 2FA/MFA - most (if not all) currently available WireGuard clients do not support real Multi-Factor Authentication/2FA - and use 2FA just as authorization to the "application" itself (and not WireGuard tunnel).

If you would like to secure your WireGuard instance try **defguard** VPN & SSO server (which is also free & open source) to get real 2FA using WireGuard PSK keys and peers configuration by defguard gateway!`,
            },
            security: {
              // md
              title: 'Security and Privacy **done right!**',
              // md
              sideText: `* Privacy requires controlling your data, thus your user data (Identity, SSO) needs to be on-premise (on your servers)
* Securing your data and applications requires authentication and authorization (SSO) with Multi-Factor Authentication, and for highest security - MFA with Hardware Security Modules
* Accessing your data and applications securely and privately requires data encryption (HTTPS) and a secure tunnel between your device and the Internet to encrypt all traffic (VPN).
* To fully trust your SSO, VPN, it needs to be Open Source`,
            },
            instances: {
              // md
              title: '**Multiple** instance & locations',
              // md
              sideText: `**defguard** (both server nad this client) support multiple instances (installations) and multiple Locations (VPN tunnels).

If you are an admin/devops - all your customers (instances) and all their tunnels (locations) can be in one place!`,
            },
            support: {
              // md
              title: '**Support us** on Github',
              // md
              text: `**defguard** is free and truly Open Source and our team has been working on it for several months. Please consider supporting us by: `,
              githubText: `staring us on`,
              githubLink: `GitHub`,
              spreadWordText: `spreading the word about:`,
              defguard: `defguard!`,
              joinMatrix: `join our Matrix server:`,
              supportUs: 'Support Us!',
            },
          },
        },
        settingsPage: {
          title: 'Settings',
          tabs: {
            global: {
              common: {
                value_in_seconds: '(seconds)',
              },
              peer_alive: {
                title: 'Session timeout',
                helper:
                  'If active connection exceeds given time without making an handshake with the server. The connection will be considered invalid and disconnected automatically.',
              },
              tray: {
                title: 'System tray',
                label: 'Tray icon theme',
                options: {
                  color: 'Color',
                  white: 'White',
                  black: 'Black',
                  gray: 'Gray',
                },
              },
              logging: {
                title: 'Logging threshold',
                warning: 'Change will take effect after client restart.',
                options: {
                  error: 'Error',
                  info: 'Info',
                  debug: 'Debug',
                  trace: 'Trace',
                },
              },
              globalLogs: {
                logSources: {
                  client: 'Client',
                  service: 'Service',
                  all: 'All',
                },
                logSourceHelper:
                  'The source of the logs. Logs can come from the Defguard client or the background Defguard service that manages VPN conncetions at the network level.',
              },
              theme: {
                title: 'Theme',
                options: {
                  light: 'Light',
                  dark: 'Dark',
                },
              },
              versionUpdate: {
                title: 'Updates',
                checkboxTitle: 'Check for updates',
              },
            },
          },
        },
        createdPage: {
          tunnel: {
            title: 'Your Tunnel Was Added Successfully',
            content:
              'Your tunnel has been successfully added. You can now connect this device, check its status and view statistics using the menu in the left sidebar.',
            controls: {
              submit: 'Add Another Tunnel',
            },
          },
          instance: {
            title: 'Your Instance Was Added Successfully',
            content:
              'Your instance has been successfully added. You can now connect this device, check its status and view statistics using the menu in the left sidebar.',
            controls: {
              submit: 'Add Another Instance',
            },
          },
        },
        instancePage: {
          title: 'Locations',
          //md
          noData: `
Currently you do not have access to any VPN Locations. This may be temporary - your administration team maybe is configuring your access policies.

If this will not change, please contact your administrator.`,
          controls: {
            connect: 'Connect',
            disconnect: 'Disconnect',
            traffic: {
              predefinedTraffic: 'Predefined traffic',
              allTraffic: 'All traffic',
              label: 'Allowed traffic',
              helper: `
                <p>
                  <b>Predefined traffic</b> - route only traffic for networks defined by Admin through this VPN location</br>
                  <b>All traffic</b> - route ALL your network traffic through this VPN location
                </p>`,
            },
          },
          header: {
            title: 'Locations',
            edit: 'Edit Instance',
            filters: {
              views: {
                grid: 'Grid View',
                detail: 'Detail View',
              },
            },
          },
          connectionLabels: {
            lastConnectedFrom: 'Last connected from',
            lastConnected: 'Last connected',
            connectedFrom: 'Connected from',
            assignedIp: 'Assigned IP',
            active: 'Active',
            neverConnected: 'Never connected',
          },
          locationNeverConnected: {
            title: 'Never Connected',
            content:
              'This device was never connected to this location, connect to view statistics and information about connection',
          },
          LocationNoStats: {
            title: 'No stats',
            content:
              'This device has no stats for this location in specified time period. Connect to location and wait for client to gather statistics.',
          },
          detailView: {
            history: {
              title: 'Connection history',
              headers: {
                date: 'Date',
                duration: 'Duration',
                connectedFrom: 'Connected from',
                upload: 'Upload',
                download: 'Download',
              },
            },
            details: {
              title: 'Details',
              logs: {
                title: 'Log',
              },
              info: {
                configuration: {
                  title: 'Device configuration',
                  pubkey: 'Public key',
                  address: 'Addresses',
                  listenPort: 'Listen port',
                },
                vpn: {
                  title: 'VPN Server Configuration',
                  pubkey: 'Public key',
                  serverAddress: 'Server Address',
                  allowedIps: 'Allowed IPs',
                  dns: 'DNS servers',
                  keepalive: 'Persistent keepalive',
                  handshake: 'Latest Handshake',
                  handshakeValue: '{seconds: number} seconds ago',
                },
              },
            },
          },
        },
        tunnelPage: {
          title: 'WireGuard Tunnels',
          header: {
            edit: 'Edit Tunnel',
          },
        },

        editTunnelPage: {
          title: 'Edit WireGuard® Tunnel',
          messages: {
            editSuccess: 'Tunnel edited',
            editError: 'Editing tunnel failed',
          },
          controls: {
            save: 'Save changes',
          },
        },
        addTunnelPage: {
          title: 'Add WireGuard® Tunnel',
          forms: {
            initTunnel: {
              title: 'Please provide Instance URL and token',
              sections: {
                vpnServer: 'VPN Server',
                advancedOptions: 'Advanced Options',
              },
              labels: {
                name: 'Tunnel Name',
                privateKey: 'Private Key',
                publicKey: 'Public Key',
                address: 'Address',
                serverPubkey: 'Public Key',
                presharedKey: 'Pre-shared Key',
                endpoint: 'VPN Server Address:Port',
                dns: 'DNS',
                allowedips: 'Allowed IPs (separate with comma)',
                persistentKeepAlive: 'Persistent Keep Alive (sec)',
                preUp: 'PreUp',
                postUp: 'PostUp',
                PreDown: 'PreDown',
                PostDown: 'PostDown',
              },
              helpers: {
                advancedOptions:
                  'Click the "Advanced Options" section to reveal additional settings for fine-tuning your WireGuard tunnel configuration. You can customize pre and post scripts, among other options.',
                name: 'A unique name for your WireGuard tunnel to identify it easily.',
                pubkey:
                  'The public key associated with the WireGuard tunnel for secure communication.',
                prvkey:
                  'The private key associated with the WireGuard tunnel for secure communication.',
                address:
                  'The IP address assigned to this WireGuard client within the VPN network.',
                serverPubkey:
                  'The public key of the WireGuard server for secure communication.',
                presharedKey: 'Optional symmetric secret key for enhanced security.',
                allowedIps:
                  'A comma-separated list of IP addresses or CIDR ranges that are allowed for communication through the tunnel.',
                endpoint:
                  'The address and port of the WireGuard server, typically in the format "hostname:port".',
                dns: 'The DNS (Domain Name System) server that the WireGuard tunnel should use for name resolution. Right now we only support DNS server IP, in the feature we will support domain search.',
                persistentKeepAlive:
                  'The interval (in seconds) for sending periodic keep-alive messages to ensure the tunnel stays active. Adjust as needed.',
                routeAllTraffic:
                  'If enabled, all network traffic will be routed through the WireGuard tunnel.',
                preUp:
                  'Shell commands or scripts to be executed before bringing up the WireGuard tunnel.',
                postUp:
                  'Shell commands or scripts to be executed after bringing up the WireGuard tunnel.',
                preDown:
                  'Shell commands or scripts to be executed before tearing down the WireGuard tunnel.',
                postDown:
                  'Shell commands or scripts to be executed after tearing down the WireGuard tunnel.',
              },
              submit: 'Add Tunnel',
              messages: {
                configError: 'Error parsing config file',
                addSuccess: 'Tunnel added',
                addError: 'Creating tunnel failed',
              },
              controls: {
                importConfig: 'Import Config File',
                generatePrvkey: 'Generate Private Key',
              },
            },
          },
          guide: {
            title: 'Adding WireGuard tunnel',
            subTitle: `<p>To establish secure communication between two or more devices over the internet create a virtual private network by configuring your tunnel.</p><p>If you don’t see options like Table or MTU it means we do not support it for now, but will be added later.</p>`,
            card: {
              title: 'Setting Up A new Tunnel:',
              content: `
                <p>1. Import Configuration File</p>
                <div>
                <ul>
                  <li> Click on the "Import Config File" button.</li>
                  <li> Navigate to configuration file using the file selection dialog.</li>
                  <li> Select the .conf file you received or created.</li>
                </ul>
                </div>
                <p>2. Or Fill in Form on the Left</p>
                <div>
                <ul>
                  <li> Enter a name for the tunnel.</li>
                  <li> Provide essential details such as the private key, public key, and endpoint (server address).</li>
                </ul>
                </div>
                <p>
                For more help, please visit defguard help (https://docs.defguard.net)
                </p>
              `,
            },
          },
        },
        addInstancePage: {
          title: 'Add Instance',
          forms: {
            initInstance: {
              title: 'Please provide Instance URL and token',
              labels: {
                url: 'Instance URL',
                token: 'Token',
              },
              submit: 'Add Instance',
            },
            device: {
              title: 'Name this device',
              labels: {
                name: 'Name',
              },
              submit: 'Finish',
              messages: {
                addSuccess: 'Device added',
              },
            },
          },
          guide: {
            title: 'Adding Instances and connecting to VPN locations',
            subTitle:
              'In order to activate this device and access all VPN locations, you must provide the URL to your defguard instance and enter the activation token.',
            card: {
              title: 'You can obtain the token by',
              content: `
                <p>1. Invoking Remote Desktop activation process yourself</p>
                <div>
                <p>
                If you have access to your defguard instance (either you are at home/office where defguard is accessible), go to defguard -> your profile -> "Add device" and choose: Activate Defguard Client. Then select if you wish to have the token sent to you by email or just copy it from defguard.
                </p>
                </div>
                <p>2. Activating remotely by your administrator</p>
                <div>
                <p>
                If you do not have access to defguard - please contact your administrator (in your onboarding message/email there were the admin contact details) and ask for Remote desktop activation - best to send you the activation email, from which you can copy the instance URL & token.
                </p>
                </div>
                <p>
                For more help, please visit defguard help (https://docs.defguard.net)
                </p>
              `,
            },
          },
        },
      },
      sideBar: {
        instances: 'defguard Instances',
        addInstance: 'Add Instance',
        addTunnel: 'Add Tunnel',
        tunnels: 'WireGuard Tunnels',
        settings: 'Settings',
        copyright: {
          copyright: `Copyright © 2023`,
          appVersion: 'Application version: {version:string}',
        },
        applicationVersion: 'Application version: ',
      },
      newApplicationVersion: {
        header: 'New version available',
        dismiss: 'Dismiss',
        releaseNotes: "See what's new",
      },
    },
    enrollment: {
      sideBar: {
        title: 'Enrollment',
        steps: {
          welcome: 'Welcome',
          verification: 'Data verification',
          password: 'Create password',
          vpn: 'Configure VPN',
          finish: 'Finish',
          mfa: 'Configure mfa',
        },
        appVersion: 'Application version',
      },
      stepsIndicator: {
        step: 'Step',
        of: 'of',
      },
      timeLeft: 'Time left',
      steps: {
        welcome: {
          title: 'Hello, {name: string}',
          explanation: `
In order to gain access to the company infrastructure, we require you to complete this enrollment process. During this process, you will need to:

1. Verify your data
2. Create your password
3. Configure VPN device

You have a time limit of **{time: string} minutes** to complete this process.
If you have any questions, please consult your assigned admin.All necessary information can be found at the bottom of the sidebar.`,
        },
        dataVerification: {
          title: 'Data verification',
          messageBox:
            'Please, check your data. If anything is wrong, notify your admin after you complete the process.',
          form: {
            fields: {
              firstName: {
                label: 'Name',
              },
              lastName: {
                label: 'Last name',
              },
              email: {
                label: 'E-mail',
              },
              phone: {
                label: 'Phone number',
              },
            },
          },
        },
        password: {
          title: 'Create password',
          form: {
            fields: {
              password: {
                label: 'Create new password',
              },
              repeat: {
                label: 'Confirm new password',
                errors: {
                  matching: `Passwords aren't matching`,
                },
              },
            },
          },
        },
        deviceSetup: {
          desktopSetup: {
            title: 'Configure this device',
            controls: {
              create: 'Configure device',
              success: 'Device is configured',
            },
            messages: {
              deviceConfigured: 'Device is configured',
            },
          },
          optionalMessage: `* This step is OPTIONAL. You can skip it if you wish. This can be configured later in defguard.`,
          cards: {
            device: {
              title: 'Configure your device for VPN',
              create: {
                submit: 'Create Configuration',
                messageBox:
                  'Please be advised that you have to download the configuration now, since we do not store your private key. After this dialog is closed, you will not be able to get your full configuration file (with private keys, only blank template).',
                form: {
                  fields: {
                    name: {
                      label: 'Device Name',
                    },
                    public: {
                      label: 'My Public Key',
                    },
                    toggle: {
                      generate: 'Generate key pair',
                      own: 'Use my own public key',
                    },
                  },
                },
              },
              config: {
                messageBox: {
                  auto: `
       <p>
          Please be advised that you have to download the configuration now,
          since <strong>we do not</strong> store your private key. After this
          dialog is closed, you <strong>will not be able</strong> to get your
          full configuration file (with private keys, only blank template).
        </p>
`,
                  manual: `
        <p>
          Please be advised that configuration provided here <strong> does not include private key and uses public key to fill it's place </strong> you will need to replace it on your own for configuration to work properly.
        </p>
`,
                },
                deviceNameLabel: 'My Device Name',
                cardTitle:
                  'Use provided configuration file below by scanning QR Code or importing it as file on your devices WireGuard app.',
                card: {
                  selectLabel: 'Config file for location',
                },
              },
            },
            guide: {
              title: 'Quick Guide',
              messageBox: 'This quick guide will help you with device configuration.',
              step: 'Step {step: number}:',
              steps: {
                wireguard: {
                  content:
                    'Download and install WireGuard client on your computer or app on phone.',
                  button: 'Download WireGuard',
                },
                downloadConfig: 'Download provided configuration file to your device.',
                addTunnel: `Open WireGuard and select "Add Tunnel" (Import tunnel(s) from file). Find your
Defguard configuration file and hit "OK". On phone use WireGuard app “+” icon and scan QR code.`,
                activate: 'Select your tunnel from the list and press "activate".',
                finish: `
**Great work - your Defguard VPN is now active!**

If you want to disengage your VPN connection, simply press "deactivate".
`,
              },
            },
          },
        },
        finish: {
          title: 'Configuration completed!',
        },
      },
    },
    sessionTimeout: {
      card: {
        header: 'Session timed out',
        message:
          'Sorry, you have exceeded the time limit to complete the process. Please try again. If you need assistance, please watch our guide or contact your administrator.',
      },
      controls: {
        back: 'Enter new token',
        contact: 'Contact admin',
      },
    },
    token: {
      card: {
        title: 'Please, enter your personal enrollment token',
        messageBox: {
          email: 'You can find token in e-mail message or use direct link.',
        },
        form: {
          errors: {
            token: {
              required: 'Token is required',
            },
          },
          fields: {
            token: {
              placeholder: 'Token',
            },
          },
          controls: {
            submit: 'Next',
          },
        },
      },
    },
  },
  modals: {
    updateInstance: {
      title: 'Update instance',
      infoMessage:
        "Enter the token sent by the administrator to update the Instance configuration.\nAlternatively, you can choose to remove this Instance entirely by clicking the 'Remove Instance' button below.",
      form: {
        fieldLabels: {
          token: 'Token',
          url: 'URL',
        },
        fieldErrors: {
          token: {
            rejected: 'Token or URL rejected.',
            instanceIsNotPresent: 'Instance for this token was not found.',
          },
        },
      },
      controls: {
        updateInstance: 'Update Instance',
        removeInstance: 'Remove Instance',
      },
      messages: {
        success: '{name: string} updated.',
        error: 'Token or URL is invalid.',
        errorInstanceNotFound: 'Instance for given token is not registered !',
      },
    },
    deleteInstance: {
      title: 'Delete instance',
      subtitle: 'Are you sure you want to delete {name: string}?',
      messages: {
        success: 'Instance deleted',
        error: 'Unexpected error occurred',
      },
      controls: {
        submit: 'Delete instance',
      },
    },
    deleteTunnel: {
      title: 'Delete tunnel',
      subtitle: 'Are you sure you want to delete {name: string}?',
      messages: {
        success: 'Tunnel deleted',
        error: 'Unexpected error occurred',
      },
      controls: {
        submit: 'Delete tunnel',
      },
    },
    mfa: {
      authentication: {
        title: 'Two-factor authentication',
        authenticatorAppDescription:
          'Paste the authentication code from your Authenticator Application.',
        emailCodeDescription:
          'Paste the authentication code that was sent to your email address.',
        mfaStartDescriptionPrimary:
          'For this connection, two-factor authentication (2FA) is mandatory.',
        mfaStartDescriptionSecondary: 'Select your preferred authentication method.',
        useAuthenticatorApp: 'Use authenticator app',
        useEmailCode: 'Use your email code',
        saveAuthenticationMethodForFutureLogins: 'Use this method for future logins',
        buttonSubmit: 'Verify',
        openidLogin: {
          description:
            'In order to connect to the VPN please login with {provider}. To do so, please click "Authenticate with {provider}" button below.',
          browserWarning:
            '**This will open a new window in your Web Browser** and automatically redirect you to the {provider} login page. After authenticating with {provider} please get back here.',
          buttonText: 'Authenticate with {provider}',
        },
        openidPending: {
          description: 'Waiting for authentication in your browser...',
          tryAgain: 'Try again',
          errorDescription:
            'There was an error during authentication. Use the try again button below to retry the authentication process.',
        },
        openidUnavailable: {
          description:
            'The OpenID authentication is currently unavailable. This may be due to a configuration issue or the Defguard instance is down. Please contact your administrator or try again later.',
          tryAgain: 'Try again',
        },
        errors: {
          mfaNotConfigured: 'Selected method has not been configured.',
          mfaStartGeneric:
            'Could not start MFA process. Please try again or contact administrator.',
          instanceNotFound: 'Could not find instance.',
          locationNotSpecified: 'Location is not specified.',
          invalidCode:
            'Error, this code is invalid, try again or contact your administrator.',
          tokenExpired: 'Token has expired. Please try to connect again.',
          authenticationTimeout:
            'Authentication took too long and timed out. Please try connecting again.',
          sessionInvalidated:
            'Error: Your login session might have been invalidated or expired. Please try again.',
        },
      },
    },
  },
} satisfies BaseTranslation;

export default en;
