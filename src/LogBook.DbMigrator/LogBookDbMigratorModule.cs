using LogBook.EntityFrameworkCore;
using Volo.Abp.Autofac;
using Volo.Abp.BackgroundJobs;
using Volo.Abp.Modularity;

namespace LogBook.DbMigrator
{
    [DependsOn(
        typeof(AbpAutofacModule),
        typeof(LogBookEntityFrameworkCoreDbMigrationsModule),
        typeof(LogBookApplicationContractsModule)
        )]
    public class LogBookDbMigratorModule : AbpModule
    {
        public override void ConfigureServices(ServiceConfigurationContext context)
        {
            Configure<AbpBackgroundJobOptions>(options => options.IsJobExecutionEnabled = false);
        }
    }
}
