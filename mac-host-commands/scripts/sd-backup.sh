#!/bin/bash
# SD 카드 자동 백업 스크립트
# LaunchAgent에서 볼륨 마운트 시 실행

LOG="$HOME/문서/시스템/로그/sd-backup.log"
DATE=$(date +%y%m%d_%H%M)

# SD 카드 볼륨 찾기 (/Volumes/에서 외장 미디어 감지)
SD_VOLUMES=()
for vol in /Volumes/*/; do
    vol_name=$(basename "$vol")
    # macOS 기본 볼륨 제외
    case "$vol_name" in
        "Macintosh HD"|"Macintosh HD - Data"|"Recovery"|"Preboot"|"VM"|"Update"|"proxmox"|"synology"|"truenas"|"nas-"*)
            continue
            ;;
    esac
    # diskutil로 외장 미디어 확인
    disk=$(diskutil info "$vol" 2>/dev/null | grep "Device Node" | awk '{print $NF}')
    if diskutil info "$vol" 2>/dev/null | grep -q "Removable Media: Removable\|Protocol: USB\|Protocol: Secure Digital"; then
        SD_VOLUMES+=("$vol")
    fi
done

if [ ${#SD_VOLUMES[@]} -eq 0 ]; then
    exit 0
fi

for SD in "${SD_VOLUMES[@]}"; do
    VOL_NAME=$(basename "$SD")
    BACKUP_DIR="$HOME/문서/미디어/사진/SD백업/${DATE}_${VOL_NAME}"

    echo "[$DATE] SD 카드 감지: $VOL_NAME ($SD)" >> "$LOG"

    # DCIM 폴더 (카메라 사진)
    if [ -d "$SD/DCIM" ]; then
        mkdir -p "$BACKUP_DIR/DCIM"
        rsync -av --progress "$SD/DCIM/" "$BACKUP_DIR/DCIM/" >> "$LOG" 2>&1
        COUNT=$(find "$BACKUP_DIR/DCIM" -type f | wc -l | tr -d ' ')
        echo "[$DATE] DCIM 백업 완료: ${COUNT}개 파일 → $BACKUP_DIR/DCIM/" >> "$LOG"

        # Synology에도 백업 (마운트 돼있으면)
        if [ -d "/Volumes/synology/백업/미러리스" ]; then
            SYNC_DIR="/Volumes/synology/백업/미러리스/SD_${DATE}_${VOL_NAME}"
            mkdir -p "$SYNC_DIR"
            rsync -av "$SD/DCIM/" "$SYNC_DIR/" >> "$LOG" 2>&1
            echo "[$DATE] Synology 백업 완료: $SYNC_DIR" >> "$LOG"
        fi
    fi

    # 영상 폴더
    for vdir in "PRIVATE" "AVCHD" "CLIP"; do
        if [ -d "$SD/$vdir" ]; then
            mkdir -p "$BACKUP_DIR/$vdir"
            rsync -av "$SD/$vdir/" "$BACKUP_DIR/$vdir/" >> "$LOG" 2>&1
            echo "[$DATE] $vdir 백업 완료" >> "$LOG"
        fi
    done

    # 전체 백업 (위에 해당 안 되는 파일)
    rsync -av --exclude="DCIM" --exclude="PRIVATE" --exclude="AVCHD" --exclude="CLIP" --exclude="System Volume Information" --exclude=".Spotlight-V100" --exclude=".fseventsd" "$SD/" "$BACKUP_DIR/기타/" >> "$LOG" 2>&1

    # macOS 알림
    osascript -e "display notification \"$VOL_NAME → $BACKUP_DIR\" with title \"SD 카드 백업 완료\" sound name \"Glass\"" 2>/dev/null

    echo "[$DATE] 백업 완료: $VOL_NAME" >> "$LOG"
    echo "---" >> "$LOG"
done
