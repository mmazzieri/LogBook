using LogBook.EntityFrameworkCore;
using Volo.Abp.Modularity;

namespace LogBook
{
    [DependsOn(
        typeof(LogBookEntityFrameworkCoreTestModule)
        )]
    public class LogBookDomainTestModule : AbpModule
    {

    }
}