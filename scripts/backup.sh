set -e

VAR=${KC_VAR:-/home/kachiclash/var}
DATE=`date +%Y%m%d`
sudo -u kachiclash tar --zstd -cf $VAR/backup/dbs-$DATE.tar.zstd $VAR/*.sqlite
