set -e

VAR=${KC_VAR:-/home/kachiclash/public/var}
DATE=`date +%Y%m%d`
mkdir -p $VAR/backup
#sudo -u kachiclash
tar --zstd -cf $VAR/backup/dbs-$DATE.tar.zstd $VAR/*.sqlite
