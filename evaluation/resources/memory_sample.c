/* macOS SDK accounting. This helper only reads the explicitly supplied PIDs. */
#include <errno.h>
#include <limits.h>
#include <libproc.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    printf("[");
    for (int i = 1; i < argc; ++i) {
        char *end = NULL;
        errno = 0;
        long pid = strtol(argv[i], &end, 10);
        if (errno || !*argv[i] || *end || pid <= 0 || pid > INT_MAX) return 2;
        struct rusage_info_v4 usage = {0};
        int result = proc_pid_rusage((int)pid, RUSAGE_INFO_V4, (rusage_info_t *)&usage);
        if (i > 1) printf(",");
        if (result) {
            printf("{\"pid\":%ld,\"error\":%d}", pid, errno);
        } else {
            printf("{\"pid\":%ld,\"resident_bytes\":%llu,"
                   "\"physical_footprint_bytes\":%llu,"
                   "\"lifetime_peak_footprint_bytes\":%llu}", pid,
                   (unsigned long long)usage.ri_resident_size,
                   (unsigned long long)usage.ri_phys_footprint,
                   (unsigned long long)usage.ri_lifetime_max_phys_footprint);
        }
    }
    printf("]\n");
    return 0;
}
