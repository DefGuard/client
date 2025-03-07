/* eslint-disable no-irregular-whitespace */
/* eslint-disable max-len */
import type { BaseTranslation } from '../i18n-types';

const fr = {
  time: {
    seconds: {
      singular: 'seconde',
      plural: 'secondes',
    },
    minutes: {
      singular: 'minute',
      plural: 'minutes',
    },
  },
  form: {
    errors: {
      invalid: 'Champ invalide',
      email: 'Entrez une adresse e-mail valide',
      required: 'Champ requis',
      minValue: 'Le champ requiert une valeur minimale de {min: number}',
      maxValue: 'Le champ ne peut pas dépasser la valeur maximale de {max: number}',
      aboveZero: 'La valeur du champ doit être supérieure à zéro',
      minLength: 'Longueur minimale de {length: number}',
      maxLength: 'Longueur maximale de {length: number}',
      specialsRequired: 'Au moins un caractère spécial',
      specialsForbidden: 'Les caractères spéciaux sont interdits',
      numberRequired: 'Au moins un chiffre requis',
      password: {
        floatingTitle: 'Veuillez corriger ce qui suit :',
      },
      oneLower: 'Au moins une lettre minuscule',
      oneUpper: 'Au moins une lettre majuscule',
      duplicatedName: 'Un appareil avec ce nom existe déjà',
    },
  },
  common: {
    controls: {
      back: 'Retour',
      next: 'Suivant',
      submit: 'Soumettre',
      cancel: 'Annuler',
      close: 'Fermer',
      reset: 'Réinitialiser',
      save: 'Enregistrer',
    },
    messages: {
      error: 'Une erreur inattendue s\'est produite !',
      errorWithMessage: 'Une erreur s\'est produite : {message}',
      tokenExpired:
        'Le jeton a expiré, veuillez contacter votre administrateur pour émettre un nouveau jeton d\'inscription',
      networkError: "Il y a eu une erreur réseau. Impossible d'atteindre le proxy.",
      configChanged:
        'La configuration pour l\'instance {instance: string} a changé. Déconnectez-vous de tous les emplacements pour appliquer les modifications.',
      deadConDropped:
        'L \'{conType: string} {name: string} a été déconnecté, tentative de reconnexion...',
      noCookie: 'Aucun defguard_proxy set-cookie reçu',
    },
  },
  components: {
    adminInfo: {
      title: 'Votre administrateur',
    },
  },
  pages: {
    client: {
      modals: {
        deadConDropped: {
          title: '{conType: string} {name: string} déconnecté',
          tunnel: 'Tunnel',
          location: 'Emplacement',
          message:
            'L \'{conType: string} {name: string} a été déconnecté, car nous avons détecté que le serveur ne répond pas avec du trafic depuis {time: number}s. Si ce message continue d\'apparaître, veuillez contacter votre administrateur',
          controls: {
            close: 'Fermer',
          },
        },
      },
      pages: {
        carouselPage: {
          slides: {
            shared: {
              //md
              isMore: '**defguard** est tout cela et plus encore !',
              githubButton: 'Visitez defguard sur',
            },
            welcome: {
              // md
              title: 'Bienvenue sur le client **defguard** !',
              instance: {
                title: 'Ajouter une instance',
                subtitle:
                  'Établissez une connexion à une instance defguard sans effort en la configurant avec un seul jeton.',
              },
              tunnel: {
                title: 'Ajouter un tunnel',
                subtitle:
                  'Utilisez-le aisément comme un client WireGuard®. Configurez votre propre tunnel ou importez un fichier de configuration.',
              },
            },
            twoFa: {
              // md
              title: 'WireGuard **2FA avec defguard**',
              // md
              sideText: `Étant donné que le protocole WireGuard ne supporte pas le 2FA/MFA - la plupart (sinon tous) des clients WireGuard actuellement disponibles ne supportent pas l\'authentification multi-facteurs/2FA réelle - et utilisent le 2FA uniquement comme autorisation à l\'"application" elle-même (et non au tunnel WireGuard).

Si vous souhaitez sécuriser votre instance WireGuard, essayez le serveur **defguard** VPN & SSO (qui est également gratuit et open source) pour obtenir un véritable 2FA en utilisant les clés PSK WireGuard et la configuration des pairs par la passerelle defguard !`,
            },
            security: {
              // md
              title: 'Sécurité et confidentialité **bien faites !**',
              // md
              sideText: `* La confidentialité nécessite le contrôle de vos données, donc vos données utilisateur (Identité, SSO) doivent être sur hebergées sur vos serveurs.
* La sécurisation de vos données et applications nécessite une authentification et une autorisation (SSO) avec une authentification multi-facteurs, et pour une sécurité plus evoluée - du MFA avec un module de sécurité matériel.
* L\'accés sécurisé et confidentielle, à vos données et applications, nécessite un chiffrement des données (HTTPS) et un tunnel sécurisé entre votre appareil et Internet pour bénéficier d'un chiffrement de bout en bout du trafic (VPN).
* Pour faire entièrement confiance à vos solutions SSO & VPN, elle doivent être open source`,
            },
            instances: {
              // md
              title: '**Multiples** instances et emplacements',
              // md
              sideText: `**defguard** (le serveur et ce client) supporte plusieurs instances (installations) et plusieurs emplacements (tunnels VPN).

Si vous êtes un administrateur/devops - tous vos clients (instances) et tous leurs tunnels (emplacements) peuvent être au même endroit !`,
            },
            support: {
              // md
              title: '**Soutenez-nous** sur Github',
              // md
              text: `**defguard** est gratuit et véritablement open source. Notre équipe travaille sur ce projet depuis plusieurs mois. N\'ésitez pas à nous soutenir : `,
              githubText: `En nous mettant des étoiles sur`,
              githubLink: `GitHub`,
              spreadWordText: `En faisant passer le mot sur :`,
              defguard: `defguard !`,
              joinMatrix: `En rejoingnant notre serveur Matrix :`,
              supportUs: 'Soutenez-nous !',
            },
          },
        },
        settingsPage: {
          title: 'Paramètres',
          tabs: {
            global: {
              common: {
                value_in_seconds: '(secondes)',
              },
              peer_alive: {
                title: 'Délai de session',
                helper:
                  'Si une connexion active dépasse le temps donné sans effectuer de handshake avec le serveur. La connexion sera considérée comme invalide et déconnectée automatiquement.',
              },
              tray: {
                title: 'Barre d\'état',
                label: 'Thème de l\'icône de la barre d\'état',
                options: {
                  color: 'Couleur',
                  white: 'Blanc',
                  black: 'Noir',
                  gray: 'Gris',
                },
              },
              logging: {
                title: 'Niveau de journalisation',
                warning: 'Le changement prendra effet après le redémarrage du client.',
                options: {
                  error: 'Erreur',
                  info: 'Info',
                  debug: 'Débogage',
                  trace: 'Trace',
                },
              },
              globalLogs: {
                logSources: {
                  client: 'Client',
                  service: 'Service',
                  all: 'Tous',
                },
                logSourceHelper:
                  'La source des journaux. Les journaux peuvent provenir du client Defguard ou du service Defguard en arrière-plan qui gère les connexions VPN au niveau du réseau.',
              },
              theme: {
                title: 'Thème',
                options: {
                  light: 'Clair',
                  dark: 'Sombre',
                },
              },
              versionUpdate: {
                title: 'Mises à jour',
                checkboxTitle: 'Vérifier les mises à jour',
              },
            },
          },
        },
        createdPage: {
          tunnel: {
            title: 'Votre tunnel a été ajouté avec succès',
            content:
              'Votre tunnel a été ajouté avec succès. Vous pouvez maintenant connecter cet appareil. Vérifiez son état et les statistiques en utilisant le menu dans la barre latérale de gauche.',
            controls: {
              submit: 'Ajouter un autre tunnel',
            },
          },
          instance: {
            title: 'Votre instance a été ajoutée avec succès',
            content:
              'Votre instance a été ajoutée avec succès. Vous pouvez maintenant connecter cet appareil. Véérifier son état et les statistiques en utilisant le menu dans la barre latérale de gauche.',
            controls: {
              submit: 'Ajouter une autre instance',
            },
          },
        },
        instancePage: {
          title: 'Emplacements',
          controls: {
            connect: 'Connecter',
            disconnect: 'Déconnecter',
            traffic: {
              predefinedTraffic: 'Trafic prédéfini',
              allTraffic: 'Tout le trafic',
              label: 'Trafic autorisé',
              helper: `
                <p>
                  <b>Trafic prédéfini</b> - router uniquement le trafic pour les réseaux définis par l'administrateur via cet emplacement VPN</br>
                  <b>Tout le trafic</b> - router TOUT votre trafic réseau via cet emplacement VPN
                </p>`,
            },
          },
          header: {
            title: 'Emplacements',
            edit: 'Modifier l\'instance',
            filters: {
              views: {
                grid: 'Vue en grille',
                detail: 'Vue détaillée',
              },
            },
          },
          connectionLabels: {
            lastConnectedFrom: 'Dernière connexion depuis',
            lastConnected: 'Dernière connexion',
            connectedFrom: 'Connecté depuis',
            assignedIp: 'IP attribuée',
            active: 'Actif',
            neverConnected: 'Jamais connecté',
          },
          locationNeverConnected: {
            title: 'Jamais connecté',
            content:
              'Cet appareil n\'a jamais été connecté à cet emplacement, connectez-vous pour voir les statistiques et les informations sur la connexion',
          },
          LocationNoStats: {
            title: 'Aucune statistique',
            content:
              'Cet appareil n\'a aucune statistique pour cet emplacement dans la période de temps spécifiée. Connectez-vous à l\'emplacement et attendez que le client rassemble les statistiques.',
          },
          detailView: {
            history: {
              title: 'Historique des connexions',
              headers: {
                date: 'Date',
                duration: 'Durée',
                connectedFrom: 'Connecté depuis',
                upload: 'Téléversement',
                download: 'Téléchargement',
              },
            },
            details: {
              title: 'Détails',
              logs: {
                title: 'Journal',
              },
              info: {
                configuration: {
                  title: 'Configuration de l\'appareil',
                  pubkey: 'Clé publique',
                  address: 'Adresses',
                  listenPort: 'Port d\'écoute',
                },
                vpn: {
                  title: 'Configuration du serveur VPN',
                  pubkey: 'Clé publique',
                  serverAddress: 'Adresse du serveur',
                  allowedIps: 'IP autorisées',
                  dns: 'Serveurs DNS',
                  keepalive: 'Keepalive persistant',
                  handshake: 'Dernier Handshake',
                  handshakeValue: 'il y a {seconds: number} secondes',
                },
              },
            },
          },
        },
        tunnelPage: {
          title: 'Tunnels WireGuard',
          header: {
            edit: 'Modifier le tunnel',
          },
        },

        editTunnelPage: {
          title: 'Modifier le tunnel WireGuard®',
          messages: {
            editSuccess: 'Tunnel modifié',
            editError: 'Échec de la modification du tunnel',
          },
          controls: {
            save: 'Enregistrer les modifications',
          },
        },
        addTunnelPage: {
          title: 'Ajouter un tunnel WireGuard®',
          forms: {
            initTunnel: {
              title: 'Veuillez fournir l\'URL de l\'instance et le jeton',
              sections: {
                vpnServer: 'Serveur VPN',
                advancedOptions: 'Options avancées',
              },
              labels: {
                name: 'Nom du tunnel',
                privateKey: 'Clé privée',
                publicKey: 'Clé publique',
                address: 'Adresse',
                serverPubkey: 'Clé publique',
                presharedKey: 'Clé pré-partagée',
                endpoint: 'Adresse:Port du serveur VPN',
                dns: 'DNS',
                allowedips: 'IP autorisées (séparées par une virgule)',
                persistentKeepAlive: 'Keep Alive persistant (sec)',
                preUp: 'PreUp',
                postUp: 'PostUp',
                PreDown: 'PreDown',
                PostDown: 'PostDown',
              },
              helpers: {
                advancedOptions:
                  'Cliquez sur la section "Options avancées" pour afficher les paramètres supplémentaires permettant d\'affiner la configuration de votre tunnel WireGuard. Parmis les options disponibles, vous pouvez ppersonnaliser les scripts pré et post',
                name: 'Un nom unique pour votre tunnel WireGuard afin de l\'identifier facilement.',
                pubkey:
                  'La clé publique associée au tunnel WireGuard pour une communication sécurisée.',
                prvkey:
                  'La clé privée associée au tunnel WireGuard pour une communication sécurisée.',
                address:
                  'L\'adresse IP attribuée à ce client WireGuard au sein du réseau VPN.',
                serverPubkey:
                  'La clé publique du serveur WireGuard pour une communication sécurisée.',
                presharedKey: 'Clé secrète symétrique optionnelle pour une sécurité renforcée.',
                allowedIps:
                  'Une liste d\'adresses IP ou de plages CIDR séparées par des virgules qui sont autorisées pour la communication via le tunnel.',
                endpoint:
                  'L\'adresse et le port du serveur WireGuard, généralement au format "nom_hôte:port".',
                dns: 'Le serveur DNS (Domain Name System) que le tunnel WireGuard doit utiliser pour la résolution de noms. Actuellement, nous ne supportons que l\'adresse IP du serveur DNS, mais nous supporterons la recherche de domaine à l\'avenir.',
                persistentKeepAlive:
                  'L\'intervalle (en secondes) pour envoyer des messages keep-alive périodiques afin de maintenir le tunnel actif. Ajustez selon les besoins.',
                routeAllTraffic:
                  'Si activé, tout le trafic réseau sera routé via le tunnel WireGuard.',
                preUp:
                  'Commandes shell ou scripts à exécuter avant de monter le tunnel WireGuard.',
                postUp:
                  'Commandes shell ou scripts à exécuter après avoir monté le tunnel WireGuard.',
                preDown:
                  'Commandes shell ou scripts à exécuter avant de démonter le tunnel WireGuard.',
                postDown:
                  'Commandes shell ou scripts à exécuter après avoir démonté le tunnel WireGuard.',
              },
              submit: 'Ajouter un tunnel',
              messages: {
                configError: 'Erreur lors de l\'analyse du fichier de configuration',
                addSuccess: 'Tunnel ajouté',
                addError: 'Échec de la création du tunnel',
              },
              controls: {
                importConfig: 'Importer un fichier de configuration',
                generatePrvkey: 'Générer une clé privée',
              },
            },
          },
          guide: {
            title: 'Ajout d\'un tunnel WireGuard',
            subTitle: `<p>Pour établir une communication sécurisée entre deux appareils ou plus sur Internet, créez un réseau privé virtuel en configurant votre tunnel.</p><p>Si vous ne voyez pas d\'options comme Table ou MTU, cela signifie que nous ne les supportons pas pour le moment, mais elles seront ajoutées plus tard.</p>`,
            card: {
              title: 'Configuration d\'un nouveau tunnel :',
              content: `
                <p>1. Importer un fichier de configuration</p>
                <div>
                <ul>
                  <li> Cliquez sur le bouton "Importer un fichier de configuration".</li>
                  <li> Accédez au fichier de configuration en utilisant la boîte de dialogue de sélection de fichier.</li>
                  <li> Sélectionnez le fichier .conf que vous avez reçu ou créé.</li>
                </ul>
                </div>
                <p>2. Ou remplissez le formulaire à gauche</p>
                <div>
                <ul>
                  <li> Entrez un nom pour le tunnel.</li>
                  <li> Fournissez des détails essentiels tels que la clé privée, la clé publique et l\'endpoint (adresse du serveur).</li>
                </ul>
                </div>
                <p>
                Pour plus d\'aide, veuillez visiter l\'aide defguard (https://docs.defguard.net)
                </p>
              `,
            },
          },
        },
        addInstancePage: {
          title: 'Ajouter une instance',
          forms: {
            initInstance: {
              title: 'Veuillez fournir l\'URL de l\'instance et le jeton',
              labels: {
                url: 'URL de l\'instance',
                token: 'Jeton',
              },
              submit: 'Ajouter une instance',
            },
            device: {
              title: 'Nommez cet appareil',
              labels: {
                name: 'Nom',
              },
              submit: 'Terminer',
              messages: {
                addSuccess: 'Appareil ajouté',
              },
            },
          },
          guide: {
            title: 'Ajout d\'instances et connexion aux emplacements VPN',
            subTitle:
              'Afin d\'activer cet appareil et d\'accéder à tous les emplacements VPN, vous devez fournir l\'URL de votre instance defguard et entrer le jeton d\'activation.',
            card: {
              title: 'Vous pouvez obtenir le jeton en',
              content: `
                <p>1. En invoquant le processus d\'activation du Bureau à distance vous-même</p>
                <div>
                <p>
                Si vous avez accès à votre instance defguard (soit vous êtes chez vous/au bureau où defguard est accessible), allez sur defguard -> votre profil -> "Ajouter un appareil" et choisissez : Activer le client Defguard. Ensuite, choisissez si vous souhaitez que le jeton vous soit envoyé par e-mail ou simplement le copier depuis defguard.
                </p>
                </div>
                <p>2. En activant à distance par votre administrateur</p>
                <div>
                <p>
                Si vous n\'avez pas accès à defguard - veuillez contacter votre administrateur (dans votre message/e-mail d\'intégration se trouvaient les coordonnées de l\'administrateur) et demandez l\'activation du bureau à distance - le mieux est de vous envoyer l\'e-mail d\'activation, à partir duquel vous pourrez copier l\'URL de l\'instance et le jeton.
                </p>
                </div>
                <p>
                Pour plus d\'aide, veuillez visiter l\'aide defguard (https://docs.defguard.net)
                </p>
              `,
            },
          },
        },
      },
      sideBar: {
        instances: 'Instances defguard',
        addInstance: 'Ajouter une instance',
        addTunnel: 'Ajouter un tunnel',
        tunnels: 'Tunnels WireGuard',
        settings: 'Paramètres',
        copyright: {
          copyright: `Copyright © 2023`,
          appVersion: 'Version de l\'application : {version:string}',
        },
        applicationVersion: 'Version de l\'application : ',
      },
      newApplicationVersion: {
        header: 'Nouvelle version disponible',
        dismiss: 'Ignorer',
        releaseNotes: 'Voir les nouveautés',
      },
    },
    enrollment: {
      sideBar: {
        title: 'Inscription',
        steps: {
          welcome: 'Bienvenue',
          verification: 'Vérification des données',
          password: 'Créer un mot de passe',
          vpn: 'Configurer le VPN',
          finish: 'Terminer',
        },
        appVersion: 'Version de l\'application',
      },
      stepsIndicator: {
        step: 'Étape',
        of: 'sur',
      },
      timeLeft: 'Temps restant',
      steps: {
        welcome: {
          title: 'Bonjour, {name: string}',
          explanation: `
Afin d'accéder à l'infrastructure de l'entreprise, vous devez compléter ce formulaire d'inscription. Au cours de ce processus, vous devrez :

1. Vérifier vos données
2. Créer votre mot de passe
3. Configurer l'appareil VPN

Vous avez un délai de **{time: string} minutes** pour le compléter.
Si vous avez des questions, veuillez consulter votre administrateur. Toutes les informations nécessaires se trouvent en bas de la barre latérale.`,
        },
        dataVerification: {
          title: 'Vérification des données',
          messageBox:
            'Veuillez vérifier vos données. Si quelque chose ne va pas, informez votre administrateur après avoir terminé.',
          form: {
            fields: {
              firstName: {
                label: 'Prénom',
              },
              lastName: {
                label: 'Nom de famille',
              },
              email: {
                label: 'E-mail',
              },
              phone: {
                label: 'Numéro de téléphone',
              },
            },
          },
        },
        password: {
          title: 'Créer un mot de passe',
          form: {
            fields: {
              password: {
                label: 'Créer un nouveau mot de passe',
              },
              repeat: {
                label: 'Confirmer le nouveau mot de passe',
                errors: {
                  matching: `Les mots de passe ne correspondent pas`,
                },
              },
            },
          },
        },
        deviceSetup: {
          desktopSetup: {
            title: 'Configurer cet appareil',
            controls: {
              create: 'Configurer l\'appareil',
              success: 'L\'appareil est configuré',
            },
            messages: {
              deviceConfigured: 'L\'appareil est configuré',
            },
          },
          optionalMessage: `* Cette étape est OPTIONNELLE. Vous pouvez l'ignorer si vous le souhaitez. Cela peut être configuré plus tard.`,
          cards: {
            device: {
              title: 'Configurer votre appareil pour le VPN',
              create: {
                submit: 'Créer la configuration',
                messageBox:
                  'Veuillez noter que vous devez télécharger la configuration, car nous ne stockons pas votre clé privée. Après la fermeture de cette boîte de dialogue, vous ne pourrez plus obtenir votre fichier de configuration complet (avec les clés privées, uniquement un modèle vierge).',
                form: {
                  fields: {
                    name: {
                      label: 'Nom de l\'appareil',
                    },
                    public: {
                      label: 'Ma clé publique',
                    },
                    toggle: {
                      generate: 'Générer une paire de clés',
                      own: 'Utiliser ma propre clé publique',
                    },
                  },
                },
              },
              config: {
                messageBox: {
                  auto: `
       <p>
          Veuillez noter que vous devez télécharger la configuration,
          car <strong>nous ne</strong> stockons pas votre clé privée. Après la fermeture de
          cette boîte de dialogue, vous <strong>ne pourrez plus</strong> obtenir
          votre fichier de configuration complet (avec les clés privées, uniquement un modèle vierge).
        </p>
`,
                  manual: `
        <p>
          Veuillez noter que la configuration fournie ici <strong>n'inclut pas la clé privée et utilise la clé publique à sa place</strong> vous devrez la remplacer vous-même pour que la configuration fonctionne correctement.
        </p>
`,
                },
                deviceNameLabel: 'Nom de mon appareil',
                cardTitle:
                  'Utilisez le fichier de configuration fourni ci-dessous en scannant le code QR ou en l\'important comme fichier sur l\'application WireGuard de votre appareil.',
                card: {
                  selectLabel: 'Fichier de configuration pour l\'emplacement',
                },
              },
            },
            guide: {
              title: 'Guide rapide',
              messageBox: 'Ce guide rapide vous aidera à configurer votre appareil.',
              step: 'Étape {step: number} :',
              steps: {
                wireguard: {
                  content:
                    'Téléchargez et installez le client WireGuard sur votre ordinateur ou l\'application sur votre téléphone.',
                  button: 'Télécharger WireGuard',
                },
                downloadConfig: 'Téléchargez le fichier de configuration fourni sur votre appareil.',
                addTunnel: `Ouvrez WireGuard et sélectionnez "Ajouter un tunnel" (Importer des tunnels depuis un fichier). Trouvez votre
fichier de configuration Defguard et cliquez sur "OK". Sur le téléphone, utilisez l\'icône “+” de l\'application WireGuard et scannez le code QR.`,
                activate: 'Sélectionnez votre tunnel dans la liste et appuyez sur "activer".',
                finish: `
**Bravo - votre VPN Defguard est maintenant actif !**

Si vous souhaitez désactiver votre connexion VPN, appuyez simplement sur "désactiver".
`,
              },
            },
          },
        },
        finish: {
          title: 'Configuration terminée !',
        },
      },
    },
    sessionTimeout: {
      card: {
        header: 'Session expirée',
        message:
          'Désolé, vous avez dépassé le délai pour compléter le formulaire. Veuillez réessayer. Si vous avez besoin d\'aide, veuillez consulter notre guide ou contacter votre administrateur.',
      },
      controls: {
        back: 'Entrer un nouveau jeton',
        contact: 'Contacter l\'administrateur',
      },
    },
    token: {
      card: {
        title: 'Veuillez entrer votre jeton d\'inscription personnel',
        messageBox: {
          email: 'Vous pouvez trouver le jeton dans le message e-mail ou utiliser le lien direct.',
        },
        form: {
          errors: {
            token: {
              required: 'Jeton requis',
            },
          },
          fields: {
            token: {
              placeholder: 'Jeton',
            },
          },
          controls: {
            submit: 'Suivant',
          },
        },
      },
    },
  },
  modals: {
    updateInstance: {
      title: 'Mettre à jour l\'instance',
      infoMessage:
        "Entrez le jeton envoyé par l'administrateur pour mettre à jour la configuration de l'Instance.\nAlternativement, vous pouvez choisir de supprimer entièrement cette Instance en cliquant sur le bouton 'Supprimer l\'Instance' ci-dessous.",
      form: {
        fieldLabels: {
          token: 'Jeton',
          url: 'URL',
        },
        fieldErrors: {
          token: {
            rejected: 'Jeton ou URL rejeté.',
            instanceIsNotPresent: 'Instance pour ce jeton non trouvée.',
          },
        },
      },
      controls: {
        updateInstance: 'Mettre à jour l\'Instance',
        removeInstance: 'Supprimer l\'Instance',
      },
      messages: {
        success: '{name: string} mis à jour.',
        error: 'Jeton ou URL invalide.',
        errorInstanceNotFound: 'Instance pour le jeton donné non enregistrée !',
      },
    },
    deleteInstance: {
      title: 'Supprimer l\'instance',
      subtitle: 'Êtes-vous sûr de vouloir supprimer {name: string} ?',
      messages: {
        success: 'Instance supprimée',
        error: 'Une erreur inattendue s\'est produite',
      },
      controls: {
        submit: 'Supprimer l\'instance',
      },
    },
    deleteTunnel: {
      title: 'Supprimer le tunnel',
      subtitle: 'Êtes-vous sûr de vouloir supprimer {name: string} ?',
      messages: {
        success: 'Tunnel supprimé',
        error: 'Une erreur inattendue s\'est produite',
      },
      controls: {
        submit: 'Supprimer le tunnel',
      },
    },
    mfa: {
      authentication: {
        title: 'Authentification à deux facteurs',
        authenticatorAppDescription:
          'Collez le code d\'authentification de votre application Authenticator.',
        emailCodeDescription:
          'Collez le code d\'authentification qui a été envoyé à votre adresse e-mail.',
        mfaStartDescriptionPrimary:
          'Pour cette connexion, l\'authentification à deux facteurs (2FA) est obligatoire.',
        mfaStartDescriptionSecondary: 'Sélectionnez votre méthode d\'authentification préférée.',
        useAuthenticatorApp: 'Utiliser l\'application authenticator',
        useEmailCode: 'Utiliser votre code e-mail',
        saveAuthenticationMethodForFutureLogins: 'Utiliser cette méthode pour les connexions futures',
        buttonSubmit: 'Vérifier',
        errors: {
          mfaNotConfigured: 'La méthode sélectionnée n\'a pas été configurée.',
          mfaStartGeneric:
            'Impossible de démarrer le processus MFA. Veuillez réessayer ou contacter l\'administrateur.',
          instanceNotFound: 'Impossible de trouver l\'instance.',
          locationNotSpecified: 'Emplacement non spécifié.',
          invalidCode:
            'Erreur, ce code est invalide, veuillez réessayer ou contacter votre administrateur.',
          tokenExpired: 'Le jeton a expiré. Veuillez essayer de vous reconnecter.',
        },
      },
    },
  },
} satisfies BaseTranslation;

export default fr;
