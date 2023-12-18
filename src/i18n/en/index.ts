/* eslint-disable no-irregular-whitespace */
/* eslint-disable max-len */
import type { BaseTranslation } from '../i18n-types';

const en = {
  time: {
    seconds: {
      singular: 'second',
      prular: 'seconds',
    },
    minutes: {
      singular: 'minute',
      prular: 'minutes',
    },
  },
  form: {
    errors: {
      invalid: 'Field is invalid',
      email: 'Enter a valid E-mail',
      required: 'Field is required',
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
    },
    messages: {
      error: 'Unexpected error occurred!',
    },
  },
  components: {
    adminInfo: {
      title: 'Your admin',
    },
  },
  pages: {
    client: {
      pages: {
        settingsPage: {
          title: 'Settings',
          tabs: {
            global: {
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
                options: {
                  error: 'Error',
                  info: 'Info',
                  debug: 'Debug',
                  trace: 'Trace',
                },
              },
              theme: {
                title: 'Theme',
                options: {
                  light: 'Light',
                  dark: 'Dark',
                },
              },
            },
          },
        },
        instancePage: {
          title: 'Locations',
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
                For more help, please visit defguard help (https://defguard.gitbook.io/)
                </p>
              `,
            },
          },
        },
      },
      sideBar: {
        instances: 'Instances',
        addInstance: 'Add Instance',
        settings: 'Settings',
        copyright: {
          copyright: `Copyright © 2023`,
          appVersion: 'Application version: {version:string}',
        },
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
3. Configurate VPN device

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
                  'Please be advised that you have to download the configuration now, since we do not store your private key. After this dialog is closed, you will not be able to get your fulll configuration file (with private keys, only blank template).',
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
          Please be advised that configuration provided here <strong> does not include private key and uses public key to fill it's place </strong> you will need to repalce it on your own for configuration to work properly.
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
                    'Download and install WireGuard client on your compputer or app on phone.',
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
      title: 'Please provided Instance token',
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
        errorInstanceNotFound: 'Intance for given token is not registered !',
      },
    },
    deleteInstance: {
      title: 'Delete instance',
      subtitle: 'Are you sure you want to delete {name: string}?',
      messages: {
        success: 'Instance deleted',
        error: 'Unexpected error occured',
      },
      controls: {
        submit: 'Delete instance',
      },
    },
  },
} satisfies BaseTranslation;

export default en;
