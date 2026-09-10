# Politique de sécurité

## Versions suivies

Mecatol est en version 0.x. Seule la dernière version publiée reçoit des
correctifs.

## Signaler une vulnérabilité

Ouvrez un **avis de sécurité privé** via l'onglet Security du dépôt GitHub
(« Report a vulnerability »). N'ouvrez pas d'issue publique pour une faille non
corrigée.

Réponse attendue sous quelques jours.

## Ce qui compte comme faille de sécurité

Mecatol est une application de bureau qui ne fait ni requête réseau ni appel
système au-delà de la lecture de ses propres données. Le périmètre est donc
étroit :

- exécution de code arbitraire depuis un fichier ouvert par l'application ;
- échappement du bac à sable du moteur de rendu ;
- dépendance connue vulnérable.

## Ce qui n'en est pas une, et compte davantage

**Une valeur normative fausse n'est pas une faille de sécurité — c'est un bug
critique.** Signalez-la par une issue publique, avec la source qui la contredit.
Elle sera traitée en priorité sur tout le reste.

Un résultat de calcul erroné peut conduire à usiner des pièces inutilisables.
C'est le risque principal de ce logiciel, et il ne relève pas de cette page mais
du [protocole de vérification des données](docs/standards.md).

## Rappel

Les tables normatives embarquées dans la version 0.1 sont marquées
`unverified` : elles ont été saisies mais pas encore confrontées à une source
primaire. L'application le signale en permanence. Ne fondez pas une décision de
fabrication sur un résultat Mecatol sans vérifier ses valeurs de base.
